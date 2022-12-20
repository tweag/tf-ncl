//! An intermediate representation for Terraform schemas. This representation is closer to the
//! generated Nickel terms while not having to deal with all possible Nickel constructs when being
//! processed.
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::Display,
};

use serde::Deserialize;

use crate::terraform::{TFBlock, TFBlockAttribute, TFBlockSchema, TFBlockType, TFSchema, TFType};

/// The entire schema, split up by configured providers. The [String] key is the local name
/// assigned to the provider.
#[derive(Debug)]
pub struct Schema {
    pub providers: HashMap<String, Provider>,
}

/// A single provider schema
#[derive(Debug)]
pub struct Provider {
    pub source: String,
    pub version: String,
    pub configuration: HashMap<String, Attribute>,
    pub data_sources: HashMap<String, Attribute>,
    pub resources: HashMap<String, Attribute>,
}

/// An attribute in an HCL block or in an HCL dictionary
#[derive(Debug, Clone)]
pub struct Attribute {
    pub description: Option<String>,
    pub optional: bool,
    pub interpolation: InterpolationStrategy,
    pub type_: Type,
}

#[derive(Debug, Copy, Clone)]
pub enum InterpolationStrategy {
    Nickel,
    Terraform { force: bool },
}

#[derive(Debug, Clone)]
pub enum Type {
    Dynamic,
    String,
    Number,
    Bool,
    List {
        min: Option<u32>,
        max: Option<u32>,
        content: Box<Type>,
    },
    Object(HashMap<String, Attribute>),
    Dictionary(Box<Type>),
}

/// A map from provider local name to [ProviderConfig]. This is ultimately determined by the choice
/// of which schemas to generate.
#[derive(Deserialize, Debug)]
pub struct Providers(pub HashMap<String, ProviderConfig>);

#[derive(Deserialize, Debug)]
pub struct ProviderConfig {
    pub source: String,
    pub version: String,
}

pub struct WithProviders<T> {
    pub providers: Providers,
    pub data: T,
}

pub trait IntoWithProviders
where
    Self: Sized,
{
    fn with_providers(self, providers: Providers) -> WithProviders<Self>;
}

impl IntoWithProviders for TFSchema {
    fn with_providers(self, providers: Providers) -> WithProviders<Self> {
        WithProviders {
            providers,
            data: self,
        }
    }
}

/// Terraform required_providers needs to be a bijection between local name and provider source
/// Returns the map provider_source -> (local_name, version) if possible.
/// TODO(vkleen) make a proper error type
fn invert_providers(schema: Providers) -> Result<HashMap<String, (String, String)>, ()> {
    let mut r = HashMap::with_capacity(schema.0.len());
    for (local_name, provider_config) in schema.0.into_iter() {
        if r.contains_key(&provider_config.source) {
            return Err(());
        }
        r.insert(
            provider_config.source,
            (local_name, provider_config.version),
        );
    }
    Ok(r)
}

fn make_configuration(provider: TFBlockSchema) -> Result<HashMap<String, Attribute>, ()> {
    provider.try_into()
}

fn make_data_sources(
    schemas: HashMap<String, TFBlockSchema>,
) -> Result<HashMap<String, Attribute>, ()> {
    Ok(values_try_into(schemas)
        .collect::<Result<HashMap<String, Attribute>, ()>>()?
        .into_iter()
        .map(|(k, v)| (k, v.add_common().into_dictionary()))
        .collect())
}

fn make_resources(
    schemas: HashMap<String, TFBlockSchema>,
) -> Result<HashMap<String, Attribute>, ()> {
    Ok(values_try_into(schemas)
        .collect::<Result<HashMap<String, Attribute>, ()>>()?
        .into_iter()
        .add_null_resource()
        .map(|(k, v)| {
            (
                k,
                v.add_common()
                    .add_lifecycle()
                    .add_provisioner()
                    .into_dictionary(),
            )
        })
        .collect())
}

impl Attribute {
    fn into_dictionary(self) -> Attribute {
        Attribute {
            type_: Type::Dictionary(Box::new(self.type_)),
            ..self
        }
    }
}

trait MetaArguments {
    fn add_lifecycle(self) -> Self;
    fn add_common(self) -> Self;
    fn add_provisioner(self) -> Self;
}

trait NullResource {
    type ResultIterator: Iterator<Item = (String, Attribute)>;
    fn add_null_resource(self) -> Self::ResultIterator;
}

impl MetaArguments for Attribute {
    fn add_lifecycle(self) -> Self {
        let Attribute { type_, .. } = self;
        let type_ = match type_ {
            Type::Object(inner) => Type::Object(inner.add_lifecycle()),
            _ => type_,
        };
        Attribute { type_, ..self }
    }

    fn add_common(self) -> Self {
        let Attribute { type_, .. } = self;
        let type_ = match type_ {
            Type::Object(inner) => Type::Object(inner.add_common()),
            _ => type_,
        };
        Attribute { type_, ..self }
    }

    fn add_provisioner(self) -> Self {
        let Attribute { type_, .. } = self;
        let type_ = match type_ {
            Type::Object(inner) => Type::Object(inner.add_provisioner()),
            _ => type_,
        };
        Attribute { type_, ..self }
    }
}

impl MetaArguments for HashMap<String, Attribute> {
    fn add_lifecycle(mut self) -> Self {
        self.extend([(
            "lifecycle".to_string(),
            Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: None,
                type_: Type::Object(
                [
                    ("create_before_destroy".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some("By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.

The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created.".to_string()),
                            type_: Type::Bool
                        }),
                    ("prevent_destroy".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some("This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration.".to_string()),
                        type_: Type::Bool
                    }),
                    ("ignore_changes".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some(r#"By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.

The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to "fix" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."#.to_string()),
                        type_: Type::List { min: None, max: None, content: Box::new(Type::String) }
                    }),
                    ("replace_triggered_by".to_string(), Attribute {
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        description: Some(r#"Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."#.to_string()),
                        type_: Type::List { min: None, max: None, content: Box::new(Type::String) }
                    }),
                ]
                .into()),
            })]);
        self
    }

    fn add_common(mut self) -> Self {
        self.extend([
            ("depends_on".to_string(), Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: Some("Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments.".to_string()),
                type_: Type::List { min: None, max: None, content: Box::new(Type::String) }
            }),
            ("provider".to_string(), Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: Some("The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference.".to_string()),
                type_: Type::String
            }),
        ]);
        self
    }

    fn add_provisioner(mut self) -> Self {
        let connection_block = (
            "connection".to_owned(),
            Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: None,
                type_: Type::Object([
                    ("type".to_owned(), Attribute {
                        description: Some(r#"The connection type. Valid values are "ssh" and "winrm". Provisioners typically assume that the remote system runs Microsoft Windows when using WinRM. Behaviors based on the SSH target_platform will force Windows-specific behavior for WinRM, unless otherwise specified."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("user".to_owned(), Attribute {
                        description: Some(r#"The user to use for the connection."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("password".to_owned(), Attribute {
                        description: Some(r#"The password to use for the connection."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("host".to_owned(), Attribute {
                        description: Some(r#"The address of the resource to connect to."#.into()),
                        optional: false,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("port".to_owned(), Attribute {
                        description: Some(r#"The port to connect to."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Number,
                    }),
                    ("timeout".to_owned(), Attribute {
                        description: Some(r#"The timeout to wait for the connection to become available. Should be provided as a string (e.g., "30s" or "5m".)"#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("script_path".to_owned(), Attribute {
                        description: Some(r#"The path used to copy scripts meant for remote execution."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("private_key".to_owned(), Attribute {
                        description: Some(r#"The contents of an SSH key to use for the connection. These can be loaded from a file on disk using the file function. This takes preference over password if provided."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("certificate".to_owned(), Attribute {
                        description: Some(r#"The contents of a signed CA Certificate. The certificate argument must be used in conjunction with a private_key. These can be loaded from a file on disk using the the file function."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("agent".to_owned(), Attribute {
                        description: Some(r#"Set to false to disable using ssh-agent to authenticate. On Windows the only supported SSH authentication agent is Pageant."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Bool,
                    }),
                    ("agent_identity".to_owned(), Attribute {
                        description: Some(r#"The preferred identity from the ssh agent for authentication."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("host_key".to_owned(), Attribute {
                        description: Some(r#"The public key from the remote host or the signing CA, used to verify the connection."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("target_platform".to_owned(), Attribute {
                        description: Some(r#"The target platform to connect to. Valid values are "windows" and "unix". If the platform is set to windows, the default script_path is c:\windows\temp\terraform_%RAND%.cmd, assuming the SSH default shell is cmd.exe. If the SSH default shell is PowerShell, set script_path to "c:/windows/temp/terraform_%RAND%.ps1""#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("https".to_owned(), Attribute {
                        description: Some(r#"Set to true to connect using HTTPS instead of HTTP."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Bool,
                    }),
                    ("insecure".to_owned(), Attribute {
                        description: Some(r#"Set to true to skip validating the HTTPS certificate chain."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Bool,
                    }),
                    ("use_ntlm".to_owned(), Attribute {
                        description: Some(r#"Set to true to use NTLM authentication rather than default (basic authentication), removing the requirement for basic authentication to be enabled within the target guest. Refer to [Authentication for Remote Connections](https://docs.microsoft.com/en-us/windows/win32/winrm/authentication-for-remote-connections) in the Windows App Development documentation for more details."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Bool,
                    }),
                    ("cacert".to_owned(), Attribute {
                        description: Some(r#"The CA certificate to validate against."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),


                    ("bastion_host".to_owned(), Attribute {
                        description: Some(r#"Setting this enables the bastion Host connection. The provisioner will connect to bastion_host first, and then connect from there to host."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("bastion_host_key".to_owned(), Attribute {
                        description: Some(r#"The public key from the remote host or the signing CA, used to verify the host connection."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("bastion_port".to_owned(), Attribute {
                        description: Some(r#"The port to use connect to the bastion host."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Number,
                    }),
                    ("bastion_user".to_owned(), Attribute {
                        description: Some(r#"The user for the connection to the bastion host."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("bastion_password".to_owned(), Attribute {
                        description: Some(r#"The password to use for the bastion host."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("bastion_private_key".to_owned(), Attribute {
                        description: Some(r#"The contents of an SSH key file to use for the bastion host. These can be loaded from a file on disk using the file function."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("bastion_certificate".to_owned(), Attribute {
                        description: Some(r#"The contents of a signed CA Certificate. The certificate argument must be used in conjunction with a bastion_private_key. These can be loaded from a file on disk using the the file function."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),


                    ("proxy_scheme".to_owned(), Attribute {
                        description: Some(r#"http or https"#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("proxy_host".to_owned(), Attribute {
                        description: Some(r#"Setting this enables the SSH over HTTP connection. This host will be connected to first, and then the host or bastion_host connection will be made from there."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("proxy_port".to_owned(), Attribute {
                        description: Some(r#"The port to use connect to the proxy host."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Number,
                    }),
                    ("proxy_user_name".to_owned(), Attribute {
                        description: Some(r#"The username to use connect to the private proxy host. This argument should be specified only if authentication is required for the HTTP Proxy server."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                    ("proxy_user_password".to_owned(), Attribute {
                        description: Some(r#"The password to use connect to the private proxy host. This argument should be specified only if authentication is required for the HTTP Proxy server."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::String,
                    }),
                ].into()),
            },
        );

        let remote_exec_type = Type::Object([
            connection_block.clone(),
            ("inline".to_owned(), Attribute {
                description: Some(r#"This is a list of command strings. The provisioner uses a default shell unless you specify a shell as the first command (eg., #!/bin/bash). You cannot provide this with script or scripts."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::List { min: None, max: None, content: Box::new(Type::String) },
            }),
            ("script".to_owned(), Attribute {
                description: Some(r#"This is a path (relative or absolute) to a local script that will be copied to the remote resource and then executed. This cannot be provided with inline or scripts."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
            ("scripts".to_owned(), Attribute {
                description: Some(r#"This is a list of paths (relative or absolute) to local scripts that will be copied to the remote resource and then executed. They are executed in the order they are provided. This cannot be provided with inline or script."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::List { min: None, max: None, content: Box::new(Type::String) },
            }),
        ].into());

        let local_exec_type = Type::Object([
            ("command".to_owned(), Attribute {
                description: Some(r#"This is the command to execute. It can be provided as a relative path to the current working directory or as an absolute path. It is evaluated in a shell, and can use environment variables or Terraform variables."#.into()),
                optional: false,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
            ("working_dir".to_owned(), Attribute {
                description: Some(r#"If provided, specifies the working directory where command will be executed. It can be provided as a relative path to the current working directory or as an absolute path. The directory must exist."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
            ("interpreter".to_owned(), Attribute {
                description: Some(r#"If provided, this is a list of interpreter arguments used to execute the command. The first argument is the interpreter itself. It can be provided as a relative path to the current working directory or as an absolute path. The remaining arguments are appended prior to the command. This allows building command lines of the form "/bin/bash", "-c", "echo foo". If interpreter is unspecified, sensible defaults will be chosen based on the system OS."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::List { min: None, max: None, content: Box::new(Type::String) },
            }),
            ("environment".to_owned(), Attribute {
                description: Some(r#"block of key value pairs representing the environment of the executed command. inherits the current process environment."#.into()),
                optional: false,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::Dictionary(Box::new(Type::String)),
            }),
            ("when".to_owned(), Attribute {
                description: Some(r#"If provided, specifies when Terraform will execute the command. For example, when = destroy specifies that the provisioner will run when the associated resource is destroyed. Refer to Destroy-Time Provisioners for details."#.into()),
                optional: false,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
        ].into());

        let file_type = Type::Object([
            ("source".to_owned(), Attribute {
                description: Some(r#"The source file or directory. Specify it either relative to the current working directory or as an absolute path. This argument cannot be combined with content."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
            ("content".to_owned(), Attribute {
                description: Some(r#"The direct content to copy on the destination. If destination is a file, the content will be written on that file. In case of a directory, a file named tf-file-content is created inside that directory. We recommend using a file as the destination when using content. This argument cannot be combined with source."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
            ("destination".to_owned(), Attribute {
                description: Some(r#"The destination path to write to on the remote system. See Destination Paths below for more information."#.into()),
                optional: false,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::String,
            }),
        ].into());

        self.extend([
           connection_block,
            ("provisioner".to_string(), Attribute {
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                description: None,
                type_: Type::Object([
                    ("file".to_owned(), Attribute {
                        description: Some("The file provisioner copies files or directories from the machine running Terraform to the newly created resource. The file provisioner supports both ssh and winrm type connections.".into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: file_type,
                    }),
                    ("local-exec".to_owned(), Attribute {
                        description: Some(r#"The local-exec provisioner invokes a local executable after a resource is created. This invokes a process on the machine running Terraform, not on the resource. See the remote-exec provisioner to run commands on the resource.

Note that even though the resource will be fully created when the provisioner is run, there is no guarantee that it will be in an operable state - for example system services such as sshd may not be started yet on compute resources."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: local_exec_type,
                    }),
                    ("remote-exec".to_owned(), Attribute {
                        description: Some("The remote-exec provisioner invokes a script on a remote resource after it is created. This can be used to run a configuration management tool, bootstrap into a cluster, etc. To invoke a local process, see the local-exec provisioner instead. The remote-exec provisioner requires a connection and supports both ssh and winrm.".into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: remote_exec_type,
                    }),
                ].into())
            }),
        ]);
        self
    }
}

impl<I> NullResource for I
where
    I: Iterator<Item = (String, Attribute)>,
{
    type ResultIterator = std::iter::Chain<I, <[(String, Attribute); 1] as IntoIterator>::IntoIter>;
    fn add_null_resource(self) -> Self::ResultIterator {
        self.chain([
            ("null_resource".to_owned(), Attribute {
                description: Some(r#"If you need to run provisioners that aren't directly associated with a specific resource, you can associate them with a null_resource.

Instances of null_resource are treated like normal resources, but they don't do anything. Like with any other resource, you can configure provisioners and connection details on a null_resource. You can also use its triggers argument and any meta-arguments to control exactly where in the dependency graph its provisioners will run."#.into()),
                optional: true,
                interpolation: InterpolationStrategy::Nickel,
                type_: Type::Object([
                    ("triggers".to_owned(), Attribute {
                        description: Some(r#"A map of values which should cause this set of provisioners to re-run. Values are meant to be interpolated references to variables or attributes of other resources."#.into()),
                        optional: true,
                        interpolation: InterpolationStrategy::Nickel,
                        type_: Type::Dictionary(Box::new(Type::String)),
                    })
                ].into()),
            })
        ])
    }
}

impl TryFrom<WithProviders<TFSchema>> for Schema {
    /// TODO(vkleen) make a proper error type
    type Error = ();

    fn try_from(s: WithProviders<TFSchema>) -> Result<Self, Self::Error> {
        let mut provider_cfgs = invert_providers(s.providers)?;
        let mut providers = HashMap::with_capacity(provider_cfgs.len());
        for (source, schema) in s.data.provider_schemas.into_iter() {
            let (local_name, version) = provider_cfgs.remove(&source).ok_or(())?;
            providers.insert(
                local_name,
                Provider {
                    source,
                    version,
                    configuration: make_configuration(schema.provider)?,
                    data_sources: make_data_sources(schema.data_source_schemas)?,
                    resources: make_resources(schema.resource_schemas)?,
                },
            );
        }
        Ok(Schema { providers })
    }
}

impl TryFrom<TFBlockAttribute> for Attribute {
    /// TODO(vkleen) make a proper error type
    type Error = ();

    fn try_from(val: TFBlockAttribute) -> Result<Self, Self::Error> {
        let (optional, interpolation) = {
            assert!(!matches!(
                (val.optional, val.required, val.computed),
                (false, false, false) | (true, true, _) | (_, true, true)
            ));
            match (val.optional, val.required, val.computed) {
                (true, false, false) => Ok((true, InterpolationStrategy::Nickel)),
                (false, true, false) => Ok((false, InterpolationStrategy::Nickel)),
                (false, false, true) => {
                    //TODO(vkleen) Once interpolation of computed fields is properly handled,
                    //these fields should no longer be optional
                    Ok((true, InterpolationStrategy::Terraform { force: true }))
                }
                (true, false, true) => {
                    //TODO(vkleen) Once interpolation of computed fields is properly handled,
                    //these fields should no longer be optional
                    Ok((true, InterpolationStrategy::Terraform { force: false }))
                }
                _ => Err(()),
            }
        }?;

        Ok(Attribute {
            description: val.description,
            optional,
            interpolation,
            type_: val.r#type.try_into()?,
        })
    }
}

impl TryFrom<TFBlockType> for Attribute {
    type Error = ();
    fn try_from(val: TFBlockType) -> Result<Self, Self::Error> {
        Ok(Attribute {
            description: val.block.description.clone(),
            optional: true,
            ///TODO(vkleen) this isn't right
            interpolation: InterpolationStrategy::Nickel,
            type_: val.try_into()?,
        })
    }
}

impl TryFrom<TFBlockType> for Type {
    type Error = ();
    fn try_from(val: TFBlockType) -> Result<Self, Self::Error> {
        use crate::terraform::TFBlockNestingMode::*;
        match val.nesting_mode {
            Single => Self::try_from(val.block),
            List | Set => Ok(Type::List {
                min: val.min_items,
                max: val.max_items,
                content: Box::new(val.block.try_into()?),
            }),
            Map => Ok(Type::Dictionary(Box::new(val.block.try_into()?))),
        }
    }
}

impl TryFrom<TFBlock> for Type {
    type Error = ();
    fn try_from(value: TFBlock) -> Result<Self, Self::Error> {
        Ok(Attribute::try_from(value)?.type_)
    }
}

impl TryFrom<TFType> for Type {
    type Error = ();
    fn try_from(val: TFType) -> Result<Self, Self::Error> {
        match val {
            TFType::Dynamic => Ok(Type::Dynamic),
            TFType::String => Ok(Type::String),
            TFType::Number => Ok(Type::Number),
            TFType::Bool => Ok(Type::Bool),
            TFType::List(inner) | TFType::Set(inner) => Ok(Type::List {
                min: None,
                max: None,
                content: Box::new(Type::try_from(*inner)?),
            }),
            TFType::Map(inner) => Ok(Type::Dictionary(Box::new(Type::try_from(*inner)?))),
            TFType::Object(inner) => {
                let inner: Result<HashMap<_, _>, _> = inner
                    .into_iter()
                    .map(|(k, v)| {
                        Ok((
                            k,
                            Attribute {
                                description: None,
                                optional: true,
                                /// Terraform does not provide a machine readable specification
                                /// for which attributes in object types are optional
                                interpolation: InterpolationStrategy::Nickel,
                                type_: v.try_into()?,
                            },
                        ))
                    })
                    .collect();
                Ok(Type::Object(inner?))
            }
            TFType::Tuple(_) => Err(()),
        }
    }
}

fn values_try_into<I, K, V, O>(x: I) -> impl Iterator<Item = Result<(K, O), V::Error>>
where
    K: Display,
    V: TryInto<O>,
    I: IntoIterator<Item = (K, V)>,
{
    x.into_iter().map(|(n, v)| v.try_into().map(|rv| (n, rv)))
}

impl TryFrom<TFBlockSchema> for Attribute {
    type Error = ();
    fn try_from(value: TFBlockSchema) -> Result<Self, Self::Error> {
        Self::try_from(value.block)
    }
}

impl TryFrom<TFBlock> for Attribute {
    type Error = ();
    fn try_from(value: TFBlock) -> Result<Self, Self::Error> {
        Ok(Attribute {
            description: value.description.clone(),
            optional: true,
            interpolation: InterpolationStrategy::Nickel,
            type_: Type::Object(value.try_into()?),
        })
    }
}

impl TryFrom<TFBlockSchema> for HashMap<String, Attribute> {
    type Error = ();
    fn try_from(value: TFBlockSchema) -> Result<Self, Self::Error> {
        Self::try_from(value.block)
    }
}

impl TryFrom<TFBlock> for HashMap<String, Attribute> {
    type Error = ();
    fn try_from(value: TFBlock) -> Result<Self, Self::Error> {
        let attribute_fields = values_try_into(value.attributes);
        let block_fields = values_try_into(value.block_types);
        attribute_fields.chain(block_fields).collect()
    }
}
