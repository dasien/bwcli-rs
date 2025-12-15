use crate::AppContext;
use crate::GlobalArgs;
use crate::commands::input::{parse_folder_input, parse_item_input};
use crate::commands::templates::get_item_template;
use crate::output::Response;
use bw_core::models::vault::CipherView;
use bw_core::services::storage::AccountManager;
use bw_core::services::vault::{
    CipherService, ConfirmationService, FieldType, ItemFilters, ValidationService, VaultError,
    VaultService, WriteService,
};
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

// Helper to create write service
fn create_write_service(ctx: &AppContext, no_interaction: bool) -> WriteService {
    let account_manager = Arc::new(AccountManager::new(ctx.storage()));
    let cipher_service = Arc::new(CipherService::new(Arc::new(ctx.sdk().clone())));
    let validation_service = Arc::new(ValidationService::new());
    let confirmation_service = Arc::new(ConfirmationService::new(no_interaction));

    WriteService::new(
        ctx.api_client(),
        ctx.storage(),
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    )
}

/// Merge updates into existing cipher view
///
/// Strategy: Update fields that are present in updates,
/// preserve fields that are not specified (null/missing).
fn merge_cipher_views(existing: CipherView, updates: CipherView) -> CipherView {
    CipherView {
        // ID must match existing
        id: existing.id,

        // These can be updated
        organization_id: updates.organization_id.or(existing.organization_id),
        folder_id: if updates.folder_id.is_some() {
            updates.folder_id
        } else {
            existing.folder_id
        },
        r#type: updates.r#type, // Type can change
        name: if updates.name.is_empty() {
            existing.name
        } else {
            updates.name
        },
        notes: updates.notes.or(existing.notes),
        favorite: updates.favorite,
        collection_ids: if updates.collection_ids.is_empty() {
            existing.collection_ids
        } else {
            updates.collection_ids
        },

        // Preserve metadata
        revision_date: existing.revision_date, // WriteService updates this
        creation_date: existing.creation_date,
        deleted_date: existing.deleted_date,
        archived_date: existing.archived_date,

        // Type-specific data - take updates if provided
        login: updates.login.or(existing.login),
        secure_note: updates.secure_note.or(existing.secure_note),
        card: updates.card.or(existing.card),
        identity: updates.identity.or(existing.identity),
        ssh_key: updates.ssh_key.or(existing.ssh_key),

        attachments: existing.attachments, // Preserve - separate management
        fields: if updates.fields.as_ref().map_or(true, |f| f.is_empty()) {
            existing.fields
        } else {
            updates.fields
        },
        password_history: existing.password_history,

        // Preserve other SDK fields
        key: existing.key,
        reprompt: existing.reprompt,
        organization_use_totp: existing.organization_use_totp,
        edit: existing.edit,
        permissions: existing.permissions,
        view_password: existing.view_password,
        local_data: existing.local_data,
    }
}

// List command implementations
pub async fn execute_list(
    cmd: ListCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
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
pub async fn execute_get(
    cmd: GetCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
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

        GetCommands::Template(template_cmd) => {
            match get_item_template(&template_cmd.template_type) {
                Ok(template) => {
                    if global_args.raw {
                        // Raw output: pretty-printed JSON
                        match serde_json::to_string_pretty(&template) {
                            Ok(json) => {
                                println!("{}", json);
                                Ok(Response::success_message(""))
                            }
                            Err(e) => Ok(Response::error(e.to_string())),
                        }
                    } else {
                        Ok(Response::success(template))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Folder(folder_cmd) => {
            let session = get_session(global_args)?;
            let folders = vault_service.list_folders(None, session).await;
            match folders {
                Ok(folders) => {
                    // FolderView.id is Option<FolderId>, compare by parsing folder_cmd.id
                    if let Some(folder) = folders.iter().find(|f| {
                        f.id.as_ref().map(|id| id.to_string()) == Some(folder_cmd.id.clone())
                    }) {
                        Ok(Response::success(folder))
                    } else {
                        Ok(Response::error(format!(
                            "Folder not found: {}",
                            folder_cmd.id
                        )))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        _ => Ok(Response::error("Not yet implemented")),
    }
}

// Create command implementations
pub async fn execute_create(
    cmd: CreateCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    match cmd {
        CreateCommands::Item(item_cmd) => {
            let session = get_session(global_args)?;

            // 1. Parse input (base64/JSON/stdin)
            let cipher_view = match parse_item_input(&item_cmd.json) {
                Ok(view) => view,
                Err(e) => return Ok(Response::error(format!("Invalid input: {}", e))),
            };

            // 2. Create via WriteService
            let write_service = create_write_service(ctx, global_args.nointeraction);
            match write_service.create_cipher(cipher_view, session).await {
                Ok(created) => {
                    // 3. Return decrypted view - created.id is Option<CipherId>
                    let vault_service = create_vault_service(ctx);
                    let id_str = created.id.map(|id| id.to_string()).unwrap_or_default();
                    match vault_service.get_item(&id_str, session).await {
                        Ok(decrypted) => Ok(Response::success(decrypted)),
                        Err(e) => Ok(Response::error(e.to_string())),
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        CreateCommands::Folder(folder_cmd) => {
            let session = get_session(global_args)?;

            // 1. Parse folder input
            let folder_input = match parse_folder_input(&folder_cmd.json) {
                Ok(input) => input,
                Err(e) => return Ok(Response::error(format!("Invalid input: {}", e))),
            };

            // 2. Create via WriteService
            let write_service = create_write_service(ctx, global_args.nointeraction);
            match write_service
                .create_folder(folder_input.name, session)
                .await
            {
                Ok(created) => {
                    // 3. Return decrypted view
                    let vault_service = create_vault_service(ctx);
                    match vault_service.list_folders(None, session).await {
                        Ok(folders) => {
                            if let Some(folder) = folders.iter().find(|f| f.id == created.id) {
                                Ok(Response::success(folder))
                            } else {
                                Ok(Response::error("Folder created but not found in cache"))
                            }
                        }
                        Err(e) => Ok(Response::error(e.to_string())),
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        CreateCommands::Attachment(_) | CreateCommands::OrgCollection(_) => {
            Ok(Response::error("Not yet implemented"))
        }
    }
}

// Edit command implementations
pub async fn execute_edit(
    cmd: EditCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    match cmd {
        EditCommands::Item(item_cmd) => {
            let session = get_session(global_args)?;
            let vault_service = create_vault_service(ctx);

            // 1. Get existing item
            let existing = match vault_service.get_item(&item_cmd.id, session).await {
                Ok(item) => item,
                Err(VaultError::ItemNotFound) => {
                    return Ok(Response::error(format!("Item not found: {}", item_cmd.id)));
                }
                Err(e) => return Ok(Response::error(e.to_string())),
            };

            // 2. Check not deleted
            if existing.deleted_date.is_some() {
                return Ok(Response::error(
                    "Cannot edit items in trash. Use 'bw restore' first.",
                ));
            }

            // 3. Parse input and merge
            let updates = match parse_item_input(&item_cmd.json) {
                Ok(view) => view,
                Err(e) => return Ok(Response::error(format!("Invalid input: {}", e))),
            };
            let merged = merge_cipher_views(existing, updates);

            // 4. Update via WriteService
            let write_service = create_write_service(ctx, global_args.nointeraction);
            match write_service
                .update_cipher(&item_cmd.id, merged, session)
                .await
            {
                Ok(updated) => {
                    // 5. Return decrypted view - updated.id is Option<CipherId>
                    let id_str = updated.id.map(|id| id.to_string()).unwrap_or_default();
                    match vault_service.get_item(&id_str, session).await {
                        Ok(decrypted) => Ok(Response::success(decrypted)),
                        Err(e) => Ok(Response::error(e.to_string())),
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        EditCommands::Folder(folder_cmd) => {
            let session = get_session(global_args)?;

            // 1. Parse folder input
            let folder_input = match parse_folder_input(&folder_cmd.json) {
                Ok(input) => input,
                Err(e) => return Ok(Response::error(format!("Invalid input: {}", e))),
            };

            // 2. Update via WriteService
            let write_service = create_write_service(ctx, global_args.nointeraction);
            match write_service
                .update_folder(&folder_cmd.id, folder_input.name, session)
                .await
            {
                Ok(_) => {
                    // 3. Return decrypted view
                    let vault_service = create_vault_service(ctx);
                    match vault_service.list_folders(None, session).await {
                        Ok(folders) => {
                            // FolderView.id is Option<FolderId>
                            if let Some(folder) = folders.iter().find(|f| {
                                f.id.as_ref().map(|id| id.to_string()) == Some(folder_cmd.id.clone())
                            }) {
                                Ok(Response::success(folder))
                            } else {
                                Ok(Response::error("Folder updated but not found in cache"))
                            }
                        }
                        Err(e) => Ok(Response::error(e.to_string())),
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        EditCommands::ItemCollections(_) | EditCommands::OrgCollection(_) => {
            Ok(Response::error("Not yet implemented"))
        }
    }
}

// Delete command implementations
pub async fn execute_delete(
    cmd: DeleteCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    // Validate session early for consistent error messages
    // (even though delete operations don't need encryption)
    let _session = get_session(global_args)?;

    match cmd {
        DeleteCommands::Item(item_cmd) => {
            let write_service = create_write_service(ctx, global_args.nointeraction);

            match write_service
                .delete_cipher(&item_cmd.id, item_cmd.permanent, global_args.nointeraction)
                .await
            {
                Ok(_) => {
                    let msg = if item_cmd.permanent {
                        "Item permanently deleted"
                    } else {
                        "Item moved to trash"
                    };
                    Ok(Response::success_message(msg))
                }
                Err(VaultError::OperationCancelled) => {
                    Ok(Response::success_message("Deletion cancelled"))
                }
                Err(VaultError::ItemNotFound) => {
                    Ok(Response::error(format!("Item not found: {}", item_cmd.id)))
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        DeleteCommands::Folder(folder_cmd) => {
            let write_service = create_write_service(ctx, global_args.nointeraction);

            match write_service.delete_folder(&folder_cmd.id).await {
                Ok(_) => Ok(Response::success_message("Folder deleted")),
                Err(VaultError::FolderNotFound) => Ok(Response::error(format!(
                    "Folder not found: {}",
                    folder_cmd.id
                ))),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        DeleteCommands::Attachment(_) | DeleteCommands::OrgCollection(_) => {
            Ok(Response::error("Not yet implemented"))
        }
    }
}

// Restore command implementation
pub async fn execute_restore(
    cmd: RestoreCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    let session = get_session(global_args)?;
    let write_service = create_write_service(ctx, global_args.nointeraction);

    match write_service.restore_cipher(&cmd.id).await {
        Ok(restored) => {
            // Return decrypted view - restored.id is Option<CipherId>
            let vault_service = create_vault_service(ctx);
            let id_str = restored.id.map(|id| id.to_string()).unwrap_or_default();
            match vault_service.get_item(&id_str, session).await {
                Ok(decrypted) => Ok(Response::success(decrypted)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Err(VaultError::ItemNotDeleted) => Ok(Response::error("Item is not in trash")),
        Err(VaultError::ItemNotFound) => Ok(Response::error(format!("Item not found: {}", cmd.id))),
        Err(e) => Ok(Response::error(e.to_string())),
    }
}

// Move command implementation
pub async fn execute_move(
    cmd: MoveCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    let session = get_session(global_args)?;
    let write_service = create_write_service(ctx, global_args.nointeraction);

    // Handle "null" string to remove from folder
    let folder_id = if cmd.folder_id == "null" {
        None
    } else {
        Some(cmd.folder_id.as_str())
    };

    match write_service
        .move_cipher(&cmd.item_id, folder_id, session)
        .await
    {
        Ok(moved) => {
            // Return decrypted view - moved.id is Option<CipherId>
            let vault_service = create_vault_service(ctx);
            let id_str = moved.id.map(|id| id.to_string()).unwrap_or_default();
            match vault_service.get_item(&id_str, session).await {
                Ok(decrypted) => Ok(Response::success(decrypted)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Err(VaultError::ItemNotFound) => {
            Ok(Response::error(format!("Item not found: {}", cmd.item_id)))
        }
        Err(VaultError::FolderNotFound) => Ok(Response::error(format!(
            "Folder not found: {}",
            cmd.folder_id
        ))),
        Err(e) => Ok(Response::error(e.to_string())),
    }
}

pub async fn execute_confirm(
    _cmd: ConfirmCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
