use crate::models::{
    ApiResponse, CreatePostRequest, CreateUserRequest, LoginRequest, LoginResponse, Post, User,
    Permission, UserRole, MenuItem, MenuItemDisplay, UserPermissions, UserSettings, UpdateSettingsRequest, UserDevice, ExportDataRequest, DeleteAccountRequest,
    CreateMenuItemRequest, UpdateMenuItemRequest, Role, CreateRoleRequest, UpdateRoleRequest, UpdateUserRoleRequest,
};
use crate::services::{PostService, UserService};
use crate::utils::{jwt, AppError};
use axum::{
    extract::{Path, State, Extension},
    http::StatusCode,
    Json,
};
use crate::database::DatabaseConnection;
use uuid::Uuid;

use validator::Validate;

// 用户相关处理器
pub async fn register(
    State(db): State<DatabaseConnection>,
    Json(user_data): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    user_data.validate()?;
    
    let user = UserService::create_user(&db, user_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(user)))
}

pub async fn login(
    State(db): State<DatabaseConnection>,
    Json(login_data): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    login_data.validate()?;
    
    let user = UserService::login(&db, login_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let token = jwt::create_token(&user)?;
    
    let response = LoginResponse { token, user };
    Ok(Json(ApiResponse::success(response)))
}

pub async fn get_profile(
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    Ok(Json(ApiResponse::success(user)))
}

// 文章相关处理器
pub async fn create_post(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Json(post_data): Json<CreatePostRequest>,
) -> Result<Json<ApiResponse<Post>>, AppError> {
    post_data.validate()?;
    
    let post = PostService::create_post(&db, post_data, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(post)))
}

pub async fn get_posts(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<Vec<Post>>>, AppError> {
    let posts = PostService::get_posts(&db).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(posts)))
}

pub async fn get_post(
    State(db): State<DatabaseConnection>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Post>>, AppError> {
    let post = PostService::get_post_by_id(&db, post_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("文章不存在"))?;
    
    Ok(Json(ApiResponse::success(post)))
}

pub async fn update_post(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Path(post_id): Path<Uuid>,
    Json(post_data): Json<CreatePostRequest>,
) -> Result<Json<ApiResponse<Post>>, AppError> {
    post_data.validate()?;
    
    let post = PostService::update_post(&db, post_id, post_data, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(post)))
}

pub async fn delete_post(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    PostService::delete_post(&db, post_id, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

// 获取用户权限
pub async fn get_user_permissions(
    State(db): State<DatabaseConnection>,
    user: User,
) -> Result<Json<ApiResponse<UserPermissions>>, StatusCode> {
    let permissions = UserPermissions {
        user_id: user.id,
        role: user.get_role(),
        permissions: user.get_permissions(),
    };

    Ok(Json(ApiResponse::success(permissions)))
}

// 获取菜单配置
pub async fn get_menu_config(
    State(_db): State<DatabaseConnection>,
    user: User,
) -> Result<Json<ApiResponse<Vec<MenuItemDisplay>>>, StatusCode> {
    let user_permissions = user.get_permissions();
    
    // 定义菜单结构
    let all_menus = vec![
        MenuItemDisplay {
            id: "dashboard".to_string(),
            title: "仪表盘".to_string(),
            path: "/dashboard".to_string(),
            icon: "DataBoard".to_string(),
            permissions: vec![Permission::Dashboard],
            children: None,
        },
        MenuItemDisplay {
            id: "posts".to_string(),
            title: "文章管理".to_string(),
            path: "/posts".to_string(),
            icon: "Document".to_string(),
            permissions: vec![Permission::PostsRead],
            children: Some(vec![
                MenuItemDisplay {
                    id: "posts-list".to_string(),
                    title: "文章列表".to_string(),
                    path: "/posts".to_string(),
                    icon: "List".to_string(),
                    permissions: vec![Permission::PostsRead],
                    children: None,
                },
                MenuItemDisplay {
                    id: "posts-create".to_string(),
                    title: "创建文章".to_string(),
                    path: "/posts/create".to_string(),
                    icon: "Plus".to_string(),
                    permissions: vec![Permission::PostsWrite],
                    children: None,
                },
            ]),
        },
        MenuItemDisplay {
            id: "users".to_string(),
            title: "用户管理".to_string(),
            path: "/users".to_string(),
            icon: "User".to_string(),
            permissions: vec![Permission::UsersRead],
            children: Some(vec![
                MenuItemDisplay {
                    id: "users-list".to_string(),
                    title: "用户列表".to_string(),
                    path: "/users".to_string(),
                    icon: "List".to_string(),
                    permissions: vec![Permission::UsersRead],
                    children: None,
                },
                MenuItemDisplay {
                    id: "users-create".to_string(),
                    title: "创建用户".to_string(),
                    path: "/users/create".to_string(),
                    icon: "Plus".to_string(),
                    permissions: vec![Permission::UsersWrite],
                    children: None,
                },
            ]),
        },
        MenuItemDisplay {
            id: "settings".to_string(),
            title: "系统设置".to_string(),
            path: "/settings".to_string(),
            icon: "Setting".to_string(),
            permissions: vec![Permission::Settings],
            children: Some(vec![
                MenuItemDisplay {
                    id: "profile".to_string(),
                    title: "个人资料".to_string(),
                    path: "/profile".to_string(),
                    icon: "UserFilled".to_string(),
                    permissions: vec![Permission::Settings],
                    children: None,
                },
                MenuItemDisplay {
                    id: "system-settings".to_string(),
                    title: "系统配置".to_string(),
                    path: "/settings".to_string(),
                    icon: "Setting".to_string(),
                    permissions: vec![Permission::SystemAdmin],
                    children: None,
                },
            ]),
        },
    ];

    // 根据用户权限过滤菜单
    let filtered_menus = filter_menus_by_permissions(all_menus, &user_permissions);

    Ok(Json(ApiResponse::success(filtered_menus)))
}

// 过滤菜单项
fn filter_menus_by_permissions(menus: Vec<MenuItemDisplay>, user_permissions: &[Permission]) -> Vec<MenuItemDisplay> {
    menus
        .into_iter()
        .filter_map(|menu| {
            // 检查主菜单权限
            if !menu.permissions.iter().any(|p| user_permissions.contains(p)) {
                return None;
            }

            // 过滤子菜单
            let filtered_children: Option<Vec<MenuItemDisplay>> = menu.children.map(|children| {
                children
                    .into_iter()
                    .filter(|child| {
                        child.permissions.iter().any(|p| user_permissions.contains(p))
                    })
                    .collect()
            });

            // 如果有子菜单但过滤后为空，则不显示主菜单
            if let Some(ref children) = filtered_children {
                if children.is_empty() {
                    return None;
                }
            }

            Some(MenuItemDisplay {
                children: filtered_children,
                ..menu
            })
        })
        .collect()
}

// 检查用户权限
pub async fn check_permission(
    State(_db): State<DatabaseConnection>,
    user: User,
    Json(permission): Json<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    if let Some(perm) = Permission::from_str(&permission) {
        let has_permission = user.has_permission(&perm);
        Ok(Json(ApiResponse::success(has_permission)))
    } else {
        Ok(Json(ApiResponse::error("无效的权限标识".to_string())))
    }
}

// 获取所有用户（管理员权限）
pub async fn get_all_users(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>, // 已通过权限中间件验证
) -> Result<Json<ApiResponse<Vec<User>>>, StatusCode> {
    match UserService::get_all_users(&db).await {
        Ok(users) => Ok(Json(ApiResponse::success(users))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// 更新用户角色（管理员权限）
pub async fn update_user_role_admin(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>, // 已通过权限中间件验证
    Path(user_id): Path<Uuid>,
    Json(role_data): Json<UpdateUserRoleRequest>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    role_data.validate()?;
    
    UserService::update_user_role(&db, user_id, &role_data.role).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    
    let user = UserService::get_user_by_id(&db, user_id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;
    
    Ok(Json(ApiResponse::success(user)))
}

// 健康检查
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

// 设置相关处理器
pub async fn get_settings(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<UserSettings>>, AppError> {
    let settings = UserService::get_user_settings(&db, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("设置不存在"))?;
    
    Ok(Json(ApiResponse::success(settings)))
}

pub async fn update_settings(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Json(settings_data): Json<UpdateSettingsRequest>,
) -> Result<Json<ApiResponse<UserSettings>>, AppError> {
    settings_data.validate()?;
    
    let settings = UserService::update_user_settings(&db, user.id, settings_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(settings)))
}

pub async fn get_user_devices(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<Vec<UserDevice>>>, AppError> {
    let devices = UserService::get_user_devices(&db, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(devices)))
}

pub async fn logout_device(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    UserService::logout_device(&db, user.id, device_id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(())))
}

pub async fn export_data(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Json(export_request): Json<ExportDataRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    export_request.validate()?;
    
    UserService::export_user_data(&db, user.id, export_request).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(())))
}

pub async fn delete_account(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Json(delete_request): Json<DeleteAccountRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    delete_request.validate()?;
    
    UserService::delete_user_account(&db, user.id, delete_request).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(())))
}

// 菜单管理相关处理器
pub async fn get_all_menu_items(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<Vec<MenuItem>>>, AppError> {
    let menu_items = UserService::get_all_menu_items(&db).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(menu_items)))
}

pub async fn get_menu_item(
    State(db): State<DatabaseConnection>,
    Path(menu_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MenuItem>>, AppError> {
    let menu_item = UserService::get_menu_item_by_id(&db, menu_id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("菜单项不存在"))?;
    
    Ok(Json(ApiResponse::success(menu_item)))
}

pub async fn create_menu_item(
    State(db): State<DatabaseConnection>,
    Json(menu_data): Json<CreateMenuItemRequest>,
) -> Result<Json<ApiResponse<MenuItem>>, AppError> {
    menu_data.validate()?;
    
    let menu_item = UserService::create_menu_item(&db, menu_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(menu_item)))
}

pub async fn update_menu_item(
    State(db): State<DatabaseConnection>,
    Path(menu_id): Path<Uuid>,
    Json(menu_data): Json<UpdateMenuItemRequest>,
) -> Result<Json<ApiResponse<MenuItem>>, AppError> {
    menu_data.validate()?;
    
    let menu_item = UserService::update_menu_item(&db, menu_id, menu_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(menu_item)))
}

pub async fn delete_menu_item(
    State(db): State<DatabaseConnection>,
    Path(menu_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    UserService::delete_menu_item(&db, menu_id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

// 角色管理相关处理器
pub async fn get_all_roles(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<Vec<Role>>>, AppError> {
    let roles = UserService::get_all_roles(&db).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(roles)))
}

pub async fn get_role(
    State(db): State<DatabaseConnection>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Role>>, AppError> {
    let role = UserService::get_role_by_id(&db, role_id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;
    
    Ok(Json(ApiResponse::success(role)))
}

pub async fn create_role(
    State(db): State<DatabaseConnection>,
    Json(role_data): Json<CreateRoleRequest>,
) -> Result<Json<ApiResponse<Role>>, AppError> {
    role_data.validate()?;
    
    let role = UserService::create_role(&db, role_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(role)))
}

pub async fn update_role(
    State(db): State<DatabaseConnection>,
    Path(role_id): Path<Uuid>,
    Json(role_data): Json<UpdateRoleRequest>,
) -> Result<Json<ApiResponse<Role>>, AppError> {
    role_data.validate()?;
    
    let role = UserService::update_role(&db, role_id, role_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(role)))
}

pub async fn delete_role(
    State(db): State<DatabaseConnection>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    UserService::delete_role(&db, role_id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

pub async fn update_user_role_new(
    State(db): State<DatabaseConnection>,
    Path(user_id): Path<Uuid>,
    Json(role_data): Json<UpdateUserRoleRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    role_data.validate()?;
    
    UserService::update_user_role(&db, user_id, &role_data.role).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}
