{
  provider.aws = [
    { region = "eu-central-1", },
    { region = "us-west-1", alias = "uw1" },
  ],
  resource."aws_account_alternate_contact"."foo" = {
    provider = "aws.uw1",
    alternate_contact_type = "OPERATIONS",
    name = "Viktor",
    title = "me",
    email_address = "me@example.com",
    phone_number = "555-1234",
    timeouts = {
      create = "1s",
    },
  },
  resource."aws_accessanalyzer_archive_rule"."example" = {
    analyzer_name = "example-analyzer",
    rule_name = "example-rule",
    filter = [
      { criteria = "error",
        exists = "true",
      },
      { criteria = "error",
        exists = "false",
      }
    ],
  },
} | import "./aws-schema.ncl"
