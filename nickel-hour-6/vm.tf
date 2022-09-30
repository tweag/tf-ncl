terraform {
  required_providers {
    azurerm = {
      source = "hashicorp/azurerm"
      version = "=3.0.0"
    }
  }
}

provider "azurerm" {
  features {}
}

variable "azure_storage_account" {
  type     = string
  nullable = false
}

variable "azure_storage_access_key" {
  type     = string
  nullable = false
}

variable "nix_nixpkgs_url" {
  type     = string
  nullable = false
}

variable "vm_admin_username" {
  type     = string
  nullable = false
  default  = "adminuser"
}

resource "azurerm_resource_group" "this" {
  name = "wsi-read-benchmark"
  location = "West Europe"
}

resource "azurerm_virtual_network" "this" {
  name = "vnet"
  location = azurerm_resource_group.this.location
  resource_group_name = azurerm_resource_group.this.name
  address_space = [ "10.0.0.0/16" ]
}

resource "azurerm_subnet" "this" {
  name = "subnet"
  resource_group_name = azurerm_resource_group.this.name
  virtual_network_name = azurerm_virtual_network.this.name
  address_prefixes = [ "10.0.2.0/24" ]
}

resource "azurerm_public_ip" "this" {
  name = "public-ip"
  location = azurerm_resource_group.this.location
  resource_group_name = azurerm_resource_group.this.name
  allocation_method = "Static"
}

resource "azurerm_network_interface" "this" {
  name = "network_interface"
  location = azurerm_resource_group.this.location
  resource_group_name = azurerm_resource_group.this.name
  ip_configuration {
    name = "internal"
    subnet_id = azurerm_subnet.this.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id = azurerm_public_ip.this.id
  }
}

resource "azurerm_linux_virtual_machine" "this" {
  name = "vm"
  resource_group_name = azurerm_resource_group.this.name
  location = azurerm_resource_group.this.location
  size = "Standard_DS1_v2"
  network_interface_ids = [
    azurerm_network_interface.this.id
  ]
  admin_username = var.vm_admin_username
  admin_ssh_key {
    username = var.vm_admin_username
    public_key = file("~/.ssh/id_rsa.pub")
  }
  os_disk {
    caching = "ReadWrite"
    storage_account_type = "Standard_LRS"
  }
  source_image_reference {
    publisher = "Canonical"
    offer = "UbuntuServer"
    sku = "16.04-LTS"
    version = "latest"
  }
  # init script
  custom_data = base64encode(templatefile("./init.sh.tftpl", {
    "azure_storage_account": var.azure_storage_account,
    "azure_storage_access_key": var.azure_storage_access_key,
    "nix_nixpkgs_url": var.nix_nixpkgs_url,
    "admin_username": var.vm_admin_username,
  }))
}

output "public_ip" {
  value = azurerm_public_ip.this.ip_address
}
