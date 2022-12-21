let remove_if_exists = fun key r =>
      if builtin.is_record r && record.has_field key r
      then record.remove key r
      else r
    in
let maybe_record_map = fun f v =>
      if builtin.is_record v
      then record.map f v
      else v
    in
let addIdField__ = fun l x =>
      x |> record.map (fun res_type r =>
      r |> record.map (fun name r => r & { "id" | force = "${%{res_type}.%{name}.id}" }))
    in
{
    Config = {
        config | Schema,
        renderable_config = mkConfig config,
        ..
    },
    Schema = 
{
  data | {
      tls_certificate | doc "
        Get information about the TLS certificates securing a host.
        
        Use this data source to get information, such as SHA1 fingerprint or serial number, about the TLS certificates that protects a URL."
        | {
        _ : {
          certificates | doc "
            The certificates protecting the site, with the root of the chain first."
            | Array ({
                cert_pem | Str | optional ,
                is_ca | Bool | optional ,
                issuer | Str | optional ,
                not_after | Str | optional ,
                not_before | Str | optional ,
                public_key_algorithm | Str | optional ,
                serial_number | Str | optional ,
                sha1_fingerprint | Str | optional ,
                signature_algorithm | Str | optional ,
                subject | Str | optional ,
                version | Num | optional ,
              })
            | optional ,
          content | doc "
            The content of the certificate in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format."
            | Str
            | optional ,
          depends_on | doc "
            Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments."
            | Array Str
            | optional ,
          id | doc "
            Unique identifier of this data source: hashing of the certificates in the chain."
            | Str
            | optional ,
          provider | doc "
            The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference."
            | Str
            | optional ,
          url | doc "
            URL of the endpoint to get the certificates from. Accepted schemes are: `https`, `tls`. For scheme `https://` it will use the HTTP protocol and apply the `proxy` configuration of the provider, if set. For scheme `tls://` it will instead use a secure TCP socket."
            | Str
            | optional ,
          verify_chain | doc "
            Whether to verify the certificate chain while parsing it or not (default: `true`)."
            | Bool
            | optional ,
        }
        }
        | optional ,
      tls_public_key | doc "
        Get a public key from a PEM-encoded private key.
        
        Use this data source to get the public key from a [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) or [OpenSSH PEM (RFC 4716)](https://datatracker.ietf.org/doc/html/rfc4716) formatted private key, for use in other resources."
        | {
        _ : {
          algorithm | doc "
            The name of the algorithm used by the given private key. Possible values are: `RSA`, `ECDSA`, `ED25519`. "
            | Str
            | optional ,
          depends_on | doc "
            Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments."
            | Array Str
            | optional ,
          id | doc "
            Unique identifier for this data source: hexadecimal representation of the SHA1 checksum of the data source."
            | Str
            | optional ,
          private_key_openssh | doc "
            The private key (in  [OpenSSH PEM (RFC 4716)](https://datatracker.ietf.org/doc/html/rfc4716) format) to extract the public key from. This is _mutually exclusive_ with `private_key_pem`. Currently-supported algorithms for keys are: `RSA`, `ECDSA`, `ED25519`. "
            | Str
            | optional ,
          private_key_pem | doc "
            The private key (in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format) to extract the public key from. This is _mutually exclusive_ with `private_key_openssh`. Currently-supported algorithms for keys are: `RSA`, `ECDSA`, `ED25519`. "
            | Str
            | optional ,
          provider | doc "
            The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference."
            | Str
            | optional ,
          public_key_fingerprint_md5 | doc "
            The fingerprint of the public key data in OpenSSH MD5 hash format, e.g. `aa:bb:cc:...`. Only available if the selected private key format is compatible, as per the rules for `public_key_openssh` and [ECDSA P224 limitations](../../docs#limitations)."
            | Str
            | optional ,
          public_key_fingerprint_sha256 | doc "
            The fingerprint of the public key data in OpenSSH SHA256 hash format, e.g. `SHA256:...`. Only available if the selected private key format is compatible, as per the rules for `public_key_openssh` and [ECDSA P224 limitations](../../docs#limitations)."
            | Str
            | optional ,
          public_key_openssh | doc "
            The public key, in  [OpenSSH PEM (RFC 4716)](https://datatracker.ietf.org/doc/html/rfc4716) format. This is also known as ['Authorized Keys'](https://www.ssh.com/academy/ssh/authorized_keys/openssh#format-of-the-authorized-keys-file) format. This is not populated for `ECDSA` with curve `P224`, as it is [not supported](../../docs#limitations). **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
          public_key_pem | doc "
            The public key, in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format. **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
        }
        }
        | optional ,
    }
    | addIdField__
    | optional ,
  output | {
    _ : {
      depends_on | Array Str | optional ,
      description | Str | optional ,
      sensitive | Bool | optional ,
      value | Str | optional ,
    }
    }
    | optional ,
  provider | {
      tls | Array ({
            alias | Str | optional ,
            proxy | Array ({
                  from_env | doc "
                    When `true` the provider will discover the proxy configuration from environment variables. This is based upon [`http.ProxyFromEnvironment`](https://pkg.go.dev/net/http#ProxyFromEnvironment) and it supports the same environment variables (default: `true`)."
                    | Bool
                    | optional ,
                  password | doc "
                    Password used for Basic authentication against the Proxy."
                    | Str
                    | optional ,
                  url | doc "
                    URL used to connect to the Proxy. Accepted schemes are: `http`, `https`, `socks5`. "
                    | Str
                    | optional ,
                  username | doc "
                    Username (or Token) used for Basic authentication against the Proxy."
                    | Str
                    | optional ,
                })
              | optional ,
          })
        | optional ,
    }
    | optional ,
  resource | {
      tls_cert_request | doc "
        Creates a Certificate Signing Request (CSR) in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format.
        
        PEM is the typical format used to request a certificate from a Certificate Authority (CA).
        
        This resource is intended to be used in conjunction with a Terraform provider for a particular certificate authority in order to provision a new certificate."
        | {
        _ : {
          cert_request_pem | doc "
            The certificate request data in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format. **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
          depends_on | doc "
            Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments."
            | Array Str
            | optional ,
          dns_names | doc "
            List of DNS names for which a certificate is being requested (i.e. certificate subjects)."
            | Array Str
            | optional ,
          id | doc "
            Unique identifier for this resource: hexadecimal representation of the SHA1 checksum of the resource."
            | Str
            | optional ,
          ip_addresses | doc "
            List of IP addresses for which a certificate is being requested (i.e. certificate subjects)."
            | Array Str
            | optional ,
          key_algorithm | doc "
            Name of the algorithm used when generating the private key provided in `private_key_pem`. "
            | Str
            | optional ,
          lifecycle | {
              create_before_destroy | doc "
                By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.
                
                The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created."
                | Bool
                | optional ,
              ignore_changes | doc "
                By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.
                
                The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to \"fix\" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."
                | Array Str
                | optional ,
              prevent_destroy | doc "
                This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration."
                | Bool
                | optional ,
              replace_triggered_by | doc "
                Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."
                | Array Str
                | optional ,
            }
            | optional ,
          private_key_pem | doc "
            Private key in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format, that the certificate will belong to. This can be read from a separate file using the [`file`](https://www.terraform.io/language/functions/file) interpolation function. Only an irreversible secure hash of the private key will be stored in the Terraform state."
            | Str ,
          provider | doc "
            The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference."
            | Str
            | optional ,
          subject | Array ({
                common_name | doc "
                  Distinguished name: `CN`"
                  | Str
                  | optional ,
                country | doc "
                  Distinguished name: `C`"
                  | Str
                  | optional ,
                locality | doc "
                  Distinguished name: `L`"
                  | Str
                  | optional ,
                organization | doc "
                  Distinguished name: `O`"
                  | Str
                  | optional ,
                organizational_unit | doc "
                  Distinguished name: `OU`"
                  | Str
                  | optional ,
                postal_code | doc "
                  Distinguished name: `PC`"
                  | Str
                  | optional ,
                province | doc "
                  Distinguished name: `ST`"
                  | Str
                  | optional ,
                serial_number | doc "
                  Distinguished name: `SERIALNUMBER`"
                  | Str
                  | optional ,
                street_address | doc "
                  Distinguished name: `STREET`"
                  | Array Str
                  | optional ,
              })
            | optional ,
          uris | doc "
            List of URIs for which a certificate is being requested (i.e. certificate subjects)."
            | Array Str
            | optional ,
        }
        }
        | optional ,
      tls_locally_signed_cert | doc "
        Creates a TLS certificate in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format using a Certificate Signing Request (CSR) and signs it with a provided (local) Certificate Authority (CA)."
        | {
        _ : {
          allowed_uses | doc "
            List of key usages allowed for the issued certificate. Values are defined in [RFC 5280](https://datatracker.ietf.org/doc/html/rfc5280) and combine flags defined by both [Key Usages](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.3) and [Extended Key Usages](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.12). Accepted values: `any_extended`, `cert_signing`, `client_auth`, `code_signing`, `content_commitment`, `crl_signing`, `data_encipherment`, `decipher_only`, `digital_signature`, `email_protection`, `encipher_only`, `ipsec_end_system`, `ipsec_tunnel`, `ipsec_user`, `key_agreement`, `key_encipherment`, `microsoft_commercial_code_signing`, `microsoft_kernel_code_signing`, `microsoft_server_gated_crypto`, `netscape_server_gated_crypto`, `ocsp_signing`, `server_auth`, `timestamping`."
            | Array Str ,
          ca_cert_pem | doc "
            Certificate data of the Certificate Authority (CA) in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format."
            | Str ,
          ca_key_algorithm | doc "
            Name of the algorithm used when generating the private key provided in `ca_private_key_pem`. "
            | Str
            | optional ,
          ca_private_key_pem | doc "
            Private key of the Certificate Authority (CA) used to sign the certificate, in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format."
            | Str ,
          cert_pem | doc "
            Certificate data in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format. **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
          cert_request_pem | doc "
            Certificate request data in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format."
            | Str ,
          depends_on | doc "
            Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments."
            | Array Str
            | optional ,
          early_renewal_hours | doc "
            The resource will consider the certificate to have expired the given number of hours before its actual expiry time. This can be useful to deploy an updated certificate in advance of the expiration of the current certificate. However, the old certificate remains valid until its true expiration time, since this resource does not (and cannot) support certificate revocation. Also, this advance update can only be performed should the Terraform configuration be applied during the early renewal period. (default: `0`)"
            | Num
            | optional ,
          id | doc "
            Unique identifier for this resource: the certificate serial number."
            | Str
            | optional ,
          is_ca_certificate | doc "
            Is the generated certificate representing a Certificate Authority (CA) (default: `false`)."
            | Bool
            | optional ,
          lifecycle | {
              create_before_destroy | doc "
                By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.
                
                The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created."
                | Bool
                | optional ,
              ignore_changes | doc "
                By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.
                
                The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to \"fix\" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."
                | Array Str
                | optional ,
              prevent_destroy | doc "
                This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration."
                | Bool
                | optional ,
              replace_triggered_by | doc "
                Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."
                | Array Str
                | optional ,
            }
            | optional ,
          provider | doc "
            The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference."
            | Str
            | optional ,
          ready_for_renewal | doc "
            Is the certificate either expired (i.e. beyond the `validity_period_hours`) or ready for an early renewal (i.e. within the `early_renewal_hours`)?"
            | Bool
            | optional ,
          set_subject_key_id | doc "
            Should the generated certificate include a [subject key identifier](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.2) (default: `false`)."
            | Bool
            | optional ,
          validity_end_time | doc "
            The time until which the certificate is invalid, expressed as an [RFC3339](https://tools.ietf.org/html/rfc3339) timestamp."
            | Str
            | optional ,
          validity_period_hours | doc "
            Number of hours, after initial issuing, that the certificate will remain valid for."
            | Num ,
          validity_start_time | doc "
            The time after which the certificate is valid, expressed as an [RFC3339](https://tools.ietf.org/html/rfc3339) timestamp."
            | Str
            | optional ,
        }
        }
        | optional ,
      tls_private_key | doc "
        Creates a PEM (and OpenSSH) formatted private key.
        
        Generates a secure private key and encodes it in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) and [OpenSSH PEM (RFC 4716)](https://datatracker.ietf.org/doc/html/rfc4716) formats. This resource is primarily intended for easily bootstrapping throwaway development environments."
        | {
        _ : {
          algorithm | doc "
            Name of the algorithm to use when generating the private key. Currently-supported values are: `RSA`, `ECDSA`, `ED25519`. "
            | Str ,
          depends_on | doc "
            Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments."
            | Array Str
            | optional ,
          ecdsa_curve | doc "
            When `algorithm` is `ECDSA`, the name of the elliptic curve to use. Currently-supported values are: `P224`, `P256`, `P384`, `P521`. (default: `P224`)."
            | Str
            | optional ,
          id | doc "
            Unique identifier for this resource: hexadecimal representation of the SHA1 checksum of the resource."
            | Str
            | optional ,
          lifecycle | {
              create_before_destroy | doc "
                By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.
                
                The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created."
                | Bool
                | optional ,
              ignore_changes | doc "
                By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.
                
                The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to \"fix\" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."
                | Array Str
                | optional ,
              prevent_destroy | doc "
                This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration."
                | Bool
                | optional ,
              replace_triggered_by | doc "
                Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."
                | Array Str
                | optional ,
            }
            | optional ,
          private_key_openssh | doc "
            Private key data in [OpenSSH PEM (RFC 4716)](https://datatracker.ietf.org/doc/html/rfc4716) format."
            | Str
            | optional ,
          private_key_pem | doc "
            Private key data in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format."
            | Str
            | optional ,
          private_key_pem_pkcs8 | doc "
            Private key data in [PKCS#8 PEM (RFC 5208)](https://datatracker.ietf.org/doc/html/rfc5208) format."
            | Str
            | optional ,
          provider | doc "
            The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference."
            | Str
            | optional ,
          public_key_fingerprint_md5 | doc "
            The fingerprint of the public key data in OpenSSH MD5 hash format, e.g. `aa:bb:cc:...`. Only available if the selected private key format is compatible, similarly to `public_key_openssh` and the [ECDSA P224 limitations](../../docs#limitations)."
            | Str
            | optional ,
          public_key_fingerprint_sha256 | doc "
            The fingerprint of the public key data in OpenSSH SHA256 hash format, e.g. `SHA256:...`. Only available if the selected private key format is compatible, similarly to `public_key_openssh` and the [ECDSA P224 limitations](../../docs#limitations)."
            | Str
            | optional ,
          public_key_openssh | doc "
             The public key data in [\"Authorized Keys\"](https://www.ssh.com/academy/ssh/authorized_keys/openssh#format-of-the-authorized-keys-file) format. This is not populated for `ECDSA` with curve `P224`, as it is [not supported](../../docs#limitations). **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
          public_key_pem | doc "
            Public key data in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format. **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
          rsa_bits | doc "
            When `algorithm` is `RSA`, the size of the generated RSA key, in bits (default: `2048`)."
            | Num
            | optional ,
        }
        }
        | optional ,
      tls_self_signed_cert | doc "
        Creates a **self-signed** TLS certificate in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format."
        | {
        _ : {
          allowed_uses | doc "
            List of key usages allowed for the issued certificate. Values are defined in [RFC 5280](https://datatracker.ietf.org/doc/html/rfc5280) and combine flags defined by both [Key Usages](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.3) and [Extended Key Usages](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.12). Accepted values: `any_extended`, `cert_signing`, `client_auth`, `code_signing`, `content_commitment`, `crl_signing`, `data_encipherment`, `decipher_only`, `digital_signature`, `email_protection`, `encipher_only`, `ipsec_end_system`, `ipsec_tunnel`, `ipsec_user`, `key_agreement`, `key_encipherment`, `microsoft_commercial_code_signing`, `microsoft_kernel_code_signing`, `microsoft_server_gated_crypto`, `netscape_server_gated_crypto`, `ocsp_signing`, `server_auth`, `timestamping`."
            | Array Str ,
          cert_pem | doc "
            Certificate data in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format. **NOTE**: the [underlying](https://pkg.go.dev/encoding/pem#Encode) [libraries](https://pkg.go.dev/golang.org/x/crypto/ssh#MarshalAuthorizedKey) that generate this value append a `\\n` at the end of the PEM. In case this disrupts your use case, we recommend using [`trimspace()`](https://www.terraform.io/language/functions/trimspace)."
            | Str
            | optional ,
          depends_on | doc "
            Use the depends_on meta-argument to handle hidden resource or module dependencies that Terraform cannot automatically infer. You only need to explicitly specify a dependency when a resource or module relies on another resource's behavior but does not access any of that resource's data in its arguments."
            | Array Str
            | optional ,
          dns_names | doc "
            List of DNS names for which a certificate is being requested (i.e. certificate subjects)."
            | Array Str
            | optional ,
          early_renewal_hours | doc "
            The resource will consider the certificate to have expired the given number of hours before its actual expiry time. This can be useful to deploy an updated certificate in advance of the expiration of the current certificate. However, the old certificate remains valid until its true expiration time, since this resource does not (and cannot) support certificate revocation. Also, this advance update can only be performed should the Terraform configuration be applied during the early renewal period. (default: `0`)"
            | Num
            | optional ,
          id | doc "
            Unique identifier for this resource: the certificate serial number."
            | Str
            | optional ,
          ip_addresses | doc "
            List of IP addresses for which a certificate is being requested (i.e. certificate subjects)."
            | Array Str
            | optional ,
          is_ca_certificate | doc "
            Is the generated certificate representing a Certificate Authority (CA) (default: `false`)."
            | Bool
            | optional ,
          key_algorithm | doc "
            Name of the algorithm used when generating the private key provided in `private_key_pem`. "
            | Str
            | optional ,
          lifecycle | {
              create_before_destroy | doc "
                By default, when Terraform must change a resource argument that cannot be updated in-place due to remote API limitations, Terraform will instead destroy the existing object and then create a new replacement object with the new configured arguments.
                
                The create_before_destroy meta-argument changes this behavior so that the new replacement object is created first, and the prior object is destroyed after the replacement is created."
                | Bool
                | optional ,
              ignore_changes | doc "
                By default, Terraform detects any difference in the current settings of a real infrastructure object and plans to update the remote object to match configuration.
                
                The ignore_changes feature is intended to be used when a resource is created with references to data that may change in the future, but should not affect said resource after its creation. In some rare cases, settings of a remote object are modified by processes outside of Terraform, which Terraform would then attempt to \"fix\" on the next run. In order to make Terraform share management responsibilities of a single object with a separate process, the ignore_changes meta-argument specifies resource attributes that Terraform should ignore when planning updates to the associated remote object."
                | Array Str
                | optional ,
              prevent_destroy | doc "
                This meta-argument, when set to true, will cause Terraform to reject with an error any plan that would destroy the infrastructure object associated with the resource, as long as the argument remains present in the configuration."
                | Bool
                | optional ,
              replace_triggered_by | doc "
                Replaces the resource when any of the referenced items change. Supply a list of expressions referencing managed resources, instances, or instance attributes. When used in a resource that uses count or for_each, you can use count.index or each.key in the expression to reference specific instances of other resources that are configured with the same count or collection."
                | Array Str
                | optional ,
            }
            | optional ,
          private_key_pem | doc "
            Private key in [PEM (RFC 1421)](https://datatracker.ietf.org/doc/html/rfc1421) format, that the certificate will belong to. This can be read from a separate file using the [`file`](https://www.terraform.io/language/functions/file) interpolation function. Only an irreversible secure hash of the private key will be stored in the Terraform state."
            | Str ,
          provider | doc "
            The provider meta-argument specifies which provider configuration to use for a resource, overriding Terraform's default behavior of selecting one based on the resource type name. Its value should be an unquoted <PROVIDER>.<ALIAS> reference."
            | Str
            | optional ,
          ready_for_renewal | doc "
            Is the certificate either expired (i.e. beyond the `validity_period_hours`) or ready for an early renewal (i.e. within the `early_renewal_hours`)?"
            | Bool
            | optional ,
          set_authority_key_id | doc "
            Should the generated certificate include an [authority key identifier](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.1): for self-signed certificates this is the same value as the [subject key identifier](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.2) (default: `false`)."
            | Bool
            | optional ,
          set_subject_key_id | doc "
            Should the generated certificate include a [subject key identifier](https://datatracker.ietf.org/doc/html/rfc5280#section-4.2.1.2) (default: `false`)."
            | Bool
            | optional ,
          subject | Array ({
                common_name | doc "
                  Distinguished name: `CN`"
                  | Str
                  | optional ,
                country | doc "
                  Distinguished name: `C`"
                  | Str
                  | optional ,
                locality | doc "
                  Distinguished name: `L`"
                  | Str
                  | optional ,
                organization | doc "
                  Distinguished name: `O`"
                  | Str
                  | optional ,
                organizational_unit | doc "
                  Distinguished name: `OU`"
                  | Str
                  | optional ,
                postal_code | doc "
                  Distinguished name: `PC`"
                  | Str
                  | optional ,
                province | doc "
                  Distinguished name: `ST`"
                  | Str
                  | optional ,
                serial_number | doc "
                  Distinguished name: `SERIALNUMBER`"
                  | Str
                  | optional ,
                street_address | doc "
                  Distinguished name: `STREET`"
                  | Array Str
                  | optional ,
              })
            | optional ,
          uris | doc "
            List of URIs for which a certificate is being requested (i.e. certificate subjects)."
            | Array Str
            | optional ,
          validity_end_time | doc "
            The time until which the certificate is invalid, expressed as an [RFC3339](https://tools.ietf.org/html/rfc3339) timestamp."
            | Str
            | optional ,
          validity_period_hours | doc "
            Number of hours, after initial issuing, that the certificate will remain valid for."
            | Num ,
          validity_start_time | doc "
            The time after which the certificate is valid, expressed as an [RFC3339](https://tools.ietf.org/html/rfc3339) timestamp."
            | Str
            | optional ,
        }
        }
        | optional ,
    }
    | addIdField__
    | optional ,
  terraform =
  {
      required_providers  | default = {
        tls  | default = {
          source  | default = "registry.terraform.io/hashicorp/tls",
          version  | default = "4.0.3",
        },
      },
    },
},
    mkConfig | Schema -> {_: Dyn}
             = (maybe_record_map (fun k v =>
                v |> maybe_record_map (fun res_type v =>
                  v |> maybe_record_map (fun res_name v =>
                    v |> remove_if_exists "id")))),
}