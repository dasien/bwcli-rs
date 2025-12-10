use crate::AppContext;
use crate::GlobalArgs;
use crate::output::Response;
use bw_core::services::storage::AccountManager;
use bw_core::services::vault::{FieldType, ItemFilters, VaultService};
use clap::{Args, Subcommand};
use std::sync::Arc;

#[derive(Subcommand)]
pub enum ListCommands {
    /// List vault items
    Items(ListItemsCommand),
    /// List folders
    Folders(ListFoldersCommand),
    /// List collections
    Collections(ListCollectionsCommand),
    /// List organizations
    Organizations(ListOrganizationsCommand),
    /// List organization collections
    #[command(name = "org-collections")]
    OrgCollections(ListOrgCollectionsCommand),
    /// List organization members
    #[command(name = "org-members")]
    OrgMembers(ListOrgMembersCommand),
}

#[derive(Args)]
pub struct ListItemsCommand {
    #[arg(long)]
    pub organizationid: Option<String>,
    #[arg(long)]
    pub collectionid: Option<String>,
    #[arg(long)]
    pub folderid: Option<String>,
    #[arg(long)]
    pub trash: bool,
    #[arg(long)]
    pub search: Option<String>,
    #[arg(long)]
    pub url: Option<String>,
}

#[derive(Args)]
pub struct ListFoldersCommand {
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Args)]
pub struct ListCollectionsCommand {
    #[arg(long)]
    pub organizationid: Option<String>,
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Args)]
pub struct ListOrganizationsCommand;

#[derive(Args)]
pub struct ListOrgCollectionsCommand {
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Args)]
pub struct ListOrgMembersCommand {
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Subcommand)]
pub enum GetCommands {
    /// Get a vault item
    Item(GetItemCommand),
    /// Get username from an item
    Username(GetUsernameCommand),
    /// Get password from an item
    Password(GetPasswordCommand),
    /// Get URI from an item
    Uri(GetUriCommand),
    /// Get TOTP code
    Totp(GetTotpCommand),
    /// Check if password is exposed
    Exposed(GetExposedCommand),
    /// Download attachment
    Attachment(GetAttachmentCommand),
    /// Get folder
    Folder(GetFolderCommand),
    /// Get collection
    Collection(GetCollectionCommand),
    /// Get organization
    #[command(name = "org")]
    Organization(GetOrganizationCommand),
    /// Get item template
    Template(GetTemplateCommand),
    /// Get account fingerprint
    Fingerprint(GetFingerprintCommand),
}

#[derive(Args)]
pub struct GetItemCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetUsernameCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetPasswordCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetUriCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetTotpCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetExposedCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetAttachmentCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long, required = true)]
    pub itemid: String,
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Args)]
pub struct GetFolderCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetCollectionCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetOrganizationCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetTemplateCommand {
    #[arg(value_name = "TYPE")]
    pub template_type: String,
}

#[derive(Args)]
pub struct GetFingerprintCommand {
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

#[derive(Subcommand)]
pub enum CreateCommands {
    /// Create vault item
    Item(CreateItemCommand),
    /// Upload attachment
    Attachment(CreateAttachmentCommand),
    /// Create folder
    Folder(CreateFolderCommand),
    /// Create organization collection
    #[command(name = "org-collection")]
    OrgCollection(CreateOrgCollectionCommand),
}

#[derive(Args)]
pub struct CreateItemCommand {
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct CreateAttachmentCommand {
    #[arg(long, required = true)]
    pub file: String,
    #[arg(long, required = true)]
    pub itemid: String,
}

#[derive(Args)]
pub struct CreateFolderCommand {
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct CreateOrgCollectionCommand {
    #[arg(value_name = "JSON")]
    pub json: String,
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Subcommand)]
pub enum EditCommands {
    /// Edit vault item
    Item(EditItemCommand),
    /// Edit item collections
    #[command(name = "item-collections")]
    ItemCollections(EditItemCollectionsCommand),
    /// Edit folder
    Folder(EditFolderCommand),
    /// Edit organization collection
    #[command(name = "org-collection")]
    OrgCollection(EditOrgCollectionCommand),
}

#[derive(Args)]
pub struct EditItemCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct EditItemCollectionsCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(value_name = "COLLECTION_IDS")]
    pub collection_ids: String,
}

#[derive(Args)]
pub struct EditFolderCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct EditOrgCollectionCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(value_name = "JSON")]
    pub json: String,
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Subcommand)]
pub enum DeleteCommands {
    /// Delete (trash) vault item
    Item(DeleteItemCommand),
    /// Delete attachment
    Attachment(DeleteAttachmentCommand),
    /// Delete folder
    Folder(DeleteFolderCommand),
    /// Delete organization collection
    #[command(name = "org-collection")]
    OrgCollection(DeleteOrgCollectionCommand),
}

#[derive(Args)]
pub struct DeleteItemCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long)]
    pub permanent: bool,
}

#[derive(Args)]
pub struct DeleteAttachmentCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long, required = true)]
    pub itemid: String,
}

#[derive(Args)]
pub struct DeleteFolderCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct DeleteOrgCollectionCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Args)]
pub struct RestoreCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct MoveCommand {
    #[arg(value_name = "ITEM_ID")]
    pub item_id: String,
    #[arg(value_name = "FOLDER_ID")]
    pub folder_id: String,
}

#[derive(Args)]
pub struct ConfirmCommand {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long, required = true)]
    pub organizationid: String,
}

/// Get the session key from global args, returning an error if not provided
fn get_session(global_args: &GlobalArgs) -> anyhow::Result<&str> {
    global_args.session.as_deref().ok_or_else(|| {
        anyhow::anyhow!("Vault is locked. Run 'bw unlock' and set BW_SESSION environment variable.")
    })
}

// Helper to create vault service
fn create_vault_service(ctx: &AppContext) -> VaultService {
    let account_manager = Arc::new(AccountManager::new(ctx.storage()));

    VaultService::new(
        ctx.api_client(),
        ctx.storage(),
        Arc::new(ctx.sdk().clone()),
        account_manager,
    )
}

// List command implementations
pub async fn execute_list(cmd: ListCommands, global_args: &GlobalArgs, ctx: &AppContext) -> anyhow::Result<Response> {
    let vault_service = create_vault_service(ctx);

    match cmd {
        ListCommands::Items(item_cmd) => {
            let session = get_session(global_args)?;
            let filters = ItemFilters {
                organization_id: item_cmd.organizationid,
                collection_id: item_cmd.collectionid,
                folder_id: item_cmd.folderid,
                search: item_cmd.search,
                url: item_cmd.url,
                trash: item_cmd.trash,
            };

            match vault_service.list_items(&filters, session).await {
                Ok(items) => Ok(Response::success(items)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::Folders(folder_cmd) => {
            let session = get_session(global_args)?;
            match vault_service
                .list_folders(folder_cmd.search.as_deref(), session)
                .await
            {
                Ok(folders) => Ok(Response::success(folders)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::Collections(collection_cmd) => {
            let session = get_session(global_args)?;
            match vault_service
                .list_collections(
                    collection_cmd.organizationid.as_deref(),
                    collection_cmd.search.as_deref(),
                    session,
                )
                .await
            {
                Ok(collections) => Ok(Response::success(collections)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::Organizations(_) => match vault_service.list_organizations().await {
            Ok(orgs) => Ok(Response::success(orgs)),
            Err(e) => Ok(Response::error(e.to_string())),
        },

        ListCommands::OrgCollections(_) | ListCommands::OrgMembers(_) => {
            Ok(Response::error("Not yet implemented"))
        }
    }
}

// Get command implementations
pub async fn execute_get(cmd: GetCommands, global_args: &GlobalArgs, ctx: &AppContext) -> anyhow::Result<Response> {
    let vault_service = create_vault_service(ctx);

    match cmd {
        GetCommands::Item(item_cmd) => {
            let session = get_session(global_args)?;
            match vault_service.get_item(&item_cmd.id, session).await {
                Ok(item) => Ok(Response::success(item)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Username(username_cmd) => {
            let session = get_session(global_args)?;
            match vault_service
                .get_field(&username_cmd.id, FieldType::Username, session)
                .await
            {
                Ok(username) => {
                    if global_args.raw {
                        println!("{}", username);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(username))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Password(password_cmd) => {
            let session = get_session(global_args)?;
            match vault_service
                .get_field(&password_cmd.id, FieldType::Password, session)
                .await
            {
                Ok(password) => {
                    if global_args.raw {
                        println!("{}", password);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(password))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Uri(uri_cmd) => {
            let session = get_session(global_args)?;
            match vault_service
                .get_field(&uri_cmd.id, FieldType::Uri, session)
                .await
            {
                Ok(uri) => {
                    if global_args.raw {
                        println!("{}", uri);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(uri))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Totp(totp_cmd) => {
            let session = get_session(global_args)?;
            match vault_service.get_totp(&totp_cmd.id, session).await {
                Ok(code) => {
                    if global_args.raw {
                        println!("{}", code);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(code))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        _ => Ok(Response::error("Not yet implemented")),
    }
}

// Stub implementations for write operations (not in scope for this enhancement)
pub async fn execute_create(
    _cmd: CreateCommands,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_edit(
    _cmd: EditCommands,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_delete(
    _cmd: DeleteCommands,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_restore(
    _cmd: RestoreCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_move(
    _cmd: MoveCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_confirm(
    _cmd: ConfirmCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
