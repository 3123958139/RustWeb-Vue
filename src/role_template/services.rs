//! 角色模块模板 service 示例。

use crate::common::models::User;
use crate::database::DatabaseConnection;

pub struct TemplateService;

impl TemplateService {
    /// 示例查询：新角色在此实现自己的业务逻辑（查库 / 调外部服务等）
    pub async fn list_items(
        _pool: &DatabaseConnection,
        _user: &User,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(vec!["模板数据项 1".to_string(), "模板数据项 2".to_string()])
    }
}
