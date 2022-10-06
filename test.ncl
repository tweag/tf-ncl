let Terraform = import "./tf.ncl" in
{
  provider.aws = {
    region = "eu-central-1",
  },
  resource."aws_default_route_table"."this" = {
    default_route_table_id = "rtb-5bd55031",
    route = [],
  },
} | Terraform
