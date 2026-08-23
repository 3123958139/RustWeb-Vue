//! OpenAPI 文档定义与导出。
//!
//! 集中注册所有 `#[utoipa::path]` 注解的处理器，生成 OpenAPI 3.1 spec。
//!
//! # 导出
//!
//! 运行 `cargo test export_openapi` 将 spec 写入 `openapi/openapi.json`（提交到仓库），
//! orval 基于该文件生成前端类型与 API 客户端。
//!
//! # 运行时
//!
//! `GET /api-docs/openapi.json` 提供实时 spec，可在浏览器查看（配合 Swagger UI 等工具）。

use utoipa::OpenApi;

/// 全量 OpenAPI 文档
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustWeb-Vue API",
        version = "1.0.0",
        description = "RustWeb-Vue 全栈管理系统 API 文档",
    ),
    paths(
        // ============ 认证（所有角色共用） ============
        crate::common::auth::handlers::login,
        crate::common::auth::handlers::get_profile,
        crate::common::auth::handlers::logout,
        // ============ meta（角色注册表等元信息） ============
        crate::roles::list_roles,
        // ============ 管理员（用户管理） ============
        crate::admin::handlers::list_users,
        crate::admin::handlers::create_user,
        crate::admin::handlers::update_user_role,
        crate::admin::handlers::delete_user,
        // 种子账号初始密码查询（GET /admin/pwd）与停用开关
        crate::admin::handlers::list_seed_passwords,
        crate::admin::handlers::get_pwd_route_status,
        crate::admin::handlers::set_pwd_route_status,
        // ============ fj200c_information（发动机监控） ============
        crate::fj200c_information::handlers::start_service_handler,
        crate::fj200c_information::handlers::stop_service_handler,
        crate::fj200c_information::handlers::service_status_handler,
        crate::fj200c_information::handlers::send_command_handler,
        crate::fj200c_information::handlers::read_config_handler,
        crate::fj200c_information::handlers::save_config_handler,
        crate::fj200c_information::handlers::list_csv_files_handler,
        crate::fj200c_information::handlers::get_csv_file_handler,
        // ============ fj200c_main（发动机测控 ECU/Adam4015/Adam4117/Dyno/Flux 五路串口） ============
        crate::fj200c_main::handlers::start_service_handler,
        crate::fj200c_main::handlers::stop_service_handler,
        crate::fj200c_main::handlers::service_status_handler,
        crate::fj200c_main::handlers::send_command_handler,
        crate::fj200c_main::handlers::read_config_handler,
        crate::fj200c_main::handlers::save_config_handler,
        crate::fj200c_main::handlers::list_csv_files_handler,
        crate::fj200c_main::handlers::get_csv_file_handler,
        crate::fj200c_main::handlers::toggle_recording_handler,
        crate::fj200c_main::handlers::toggle_simulation_handler,
        crate::fj200c_main::handlers::set_theme_handler,
        crate::fj200c_main::handlers::get_experiment_handler,
        crate::fj200c_main::handlers::save_experiment_handler,
        crate::fj200c_main::handlers::generate_report_handler,
        crate::fj200c_main::handlers::get_help_handler,
        // ============ mario（超级马里奥复刻游戏） ============
        crate::mario::handlers::list_scores,
        crate::mario::handlers::submit_score,
        crate::mario::handlers::get_stats,
    ),
    components(
        schemas(
            // 公共数据模型（ApiResponse<T> 泛型实例由 utoipa 自动收集，无需显式列出）
            crate::common::models::User,
            crate::common::models::LoginRequest,
            crate::common::models::LoginResponse,
            crate::common::models::LogoutRequest,
            crate::common::models::CreateUserRequest,
            crate::common::models::UpdateUserRoleRequest,
            crate::common::models::UserSettings,
            crate::common::models::Permission,
            // 种子账号初始密码
            crate::admin::handlers::SeedPasswordInfo,
            crate::admin::handlers::PwdRouteStatus,
            // meta（角色注册表）
            crate::roles::RoleInfo,
            // 公共响应 DTO
            crate::common::dto::ServiceStatus,
            crate::common::dto::SentResult,
            crate::common::dto::SavedResult,
            crate::common::dto::ConfigContent,
            crate::common::dto::CsvFileList,
            crate::common::dto::CsvFileContent,
            // fj200c_information 请求体
            crate::fj200c_information::handlers::SendCommandRequest,
            crate::fj200c_information::handlers::SaveConfigRequest,
            // fj200c_main 数据模型
            crate::fj200c_main::types::EcuFields,
            crate::fj200c_main::types::FaultCodeFlags,
            crate::fj200c_main::types::Adam4015Fields,
            crate::fj200c_main::types::Adam4117Fields,
            crate::fj200c_main::types::DynoFields,
            crate::fj200c_main::types::FluxFields,
            crate::fj200c_main::types::ChannelData,
            crate::fj200c_main::types::ExperimentInfo,
            crate::fj200c_main::types::ReportOutput,
            crate::fj200c_main::types::PerformanceRow,
            crate::fj200c_main::types::StandardRow,
            crate::fj200c_main::types::DesignPointRow,
            // fj200c_main 请求/响应 DTO
            crate::fj200c_main::handlers::SendCommandRequest,
            crate::fj200c_main::handlers::SaveConfigRequest,
            crate::fj200c_main::handlers::GenerateReportRequest,
            crate::fj200c_main::handlers::ThemeRequest,
            crate::fj200c_main::handlers::RecordingState,
            crate::fj200c_main::handlers::SimulationState,
            crate::fj200c_main::handlers::ThemeState,
            // mario 模型（超级马里奥复刻游戏）
            crate::mario::models::MarioScore,
            crate::mario::models::ScoreList,
            crate::mario::models::MarioStats,
            crate::mario::models::SubmitScoreRequest,
        )
    ),
)]
pub struct ApiDoc;

/// 生成 spec 并写入 `openapi/openapi.json`
/// （供 `npm run gen:api` 的 `cargo test export_openapi` 使用）
#[allow(dead_code)]
pub fn export_spec() -> String {
    let spec = ApiDoc::openapi().to_pretty_json().unwrap();
    std::fs::create_dir_all("openapi").unwrap();
    std::fs::write("openapi/openapi.json", &spec).unwrap();
    spec
}

/// 运行时 OpenAPI spec 端点（`GET /api-docs/openapi.json`）
pub async fn openapi_json() -> axum::Json<serde_json::Value> {
    let spec = ApiDoc::openapi().to_pretty_json().unwrap();
    let doc: serde_json::Value = serde_json::from_str(&spec).unwrap();
    axum::Json(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 导出 OpenAPI spec 并断言关键路径完整
    #[test]
    fn export_openapi() {
        let spec = export_spec();
        let doc: serde_json::Value = serde_json::from_str(&spec).unwrap();
        let paths = doc["paths"].as_object().unwrap();

        for path in [
            "/api/auth/login",
            "/api/auth/profile",
            "/api/auth/logout",
            "/api/meta/roles",
            "/admin/pwd",
            "/api/users",
            "/api/users/{id}/role",
            "/api/users/{id}",
            "/api/users/settings/pwd-route",
            "/api/fj200c_information/service/start",
            "/api/fj200c_information/service/stop",
            "/api/fj200c_information/service/status",
            "/api/fj200c_information/service/command",
            "/api/fj200c_information/config",
            "/api/fj200c_information/csv/files",
            "/api/fj200c_information/csv/{name}",
            // fj200c_main：13 个端点
            "/api/fj200c_main/service/start",
            "/api/fj200c_main/service/stop",
            "/api/fj200c_main/service/status",
            "/api/fj200c_main/service/command",
            "/api/fj200c_main/config",
            "/api/fj200c_main/csv/files",
            "/api/fj200c_main/csv/{name}",
            "/api/fj200c_main/recording/toggle",
            "/api/fj200c_main/simulation/toggle",
            "/api/fj200c_main/theme/set",
            "/api/fj200c_main/experiment",
            "/api/fj200c_main/report",
            "/api/fj200c_main/help",
            // mario：2 个唯一路径（scores GET/POST + stats GET）
            "/api/mario/scores",
            "/api/mario/stats",
        ] {
            assert!(paths.contains_key(path), "缺少路径: {}", path);
        }

        // 唯一路径数：auth 3 + meta 1 + seed pwd 1 + users 4（settings/pwd-route 占 1）
        //           + fj200c_information 7 + fj200c_main 13 + mario 2（scores/stats）= 31
        assert_eq!(paths.len(), 31, "路径数量与预期不符");

        // 断言 operationId 存在（orval 依赖它生成函数名）
        let mut operations = 0;
        for op in paths.values() {
            for (method, operation) in op.as_object().unwrap() {
                if ["get", "post", "put", "delete"].contains(&method.as_str()) {
                    assert!(
                        operation["operationId"].is_string(),
                        "缺少 operationId: {} {}",
                        method,
                        operation
                    );
                    operations += 1;
                }
            }
        }
        // 37 个 HTTP 操作（不含 WebSocket）
        assert_eq!(operations, 37, "操作数量与预期不符");
    }
}
