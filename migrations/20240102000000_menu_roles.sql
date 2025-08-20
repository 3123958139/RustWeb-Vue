-- 创建菜单表
CREATE TABLE IF NOT EXISTS menu_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(100) NOT NULL,
    path VARCHAR(200),
    icon VARCHAR(50),
    parent_id UUID REFERENCES menu_items(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    permissions TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建角色表
CREATE TABLE IF NOT EXISTS roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT NOT NULL,
    permissions TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建用户角色关联表
CREATE TABLE IF NOT EXISTS user_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, role_id)
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_menu_items_parent_id ON menu_items(parent_id);
CREATE INDEX IF NOT EXISTS idx_menu_items_sort_order ON menu_items(sort_order);
CREATE INDEX IF NOT EXISTS idx_menu_items_enabled ON menu_items(enabled);
CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name);
CREATE INDEX IF NOT EXISTS idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_roles_role_id ON user_roles(role_id);

-- 插入默认角色
INSERT INTO roles (name, description, permissions) VALUES
('admin', '系统管理员，拥有所有权限', ARRAY['dashboard', 'posts:read', 'posts:write', 'posts:delete', 'users:read', 'users:write', 'users:delete', 'settings', 'system:admin']),
('moderator', '版主，负责内容审核和用户管理', ARRAY['dashboard', 'posts:read', 'posts:write', 'posts:delete', 'users:read', 'settings']),
('user', '普通用户，基础功能访问', ARRAY['dashboard', 'posts:read', 'posts:write', 'settings'])
ON CONFLICT (name) DO NOTHING;

-- 插入默认菜单项
INSERT INTO menu_items (title, path, icon, parent_id, sort_order, enabled, permissions) VALUES
('仪表盘', '/dashboard', 'DataBoard', NULL, 1, true, ARRAY['dashboard']),
('文章管理', '/posts', 'Document', NULL, 2, true, ARRAY['posts:read']),
('文章列表', '/posts', 'List', (SELECT id FROM menu_items WHERE title = '文章管理'), 1, true, ARRAY['posts:read']),
('创建文章', '/posts/create', 'Plus', (SELECT id FROM menu_items WHERE title = '文章管理'), 2, true, ARRAY['posts:write']),
('用户管理', '/users', 'User', NULL, 3, true, ARRAY['users:read']),
('系统设置', '/settings', 'Setting', NULL, 4, true, ARRAY['settings']),
('个人资料', '/profile', 'UserFilled', (SELECT id FROM menu_items WHERE title = '系统设置'), 1, true, ARRAY['settings']),
('菜单管理', '/menu-management', 'List', NULL, 5, true, ARRAY['system:admin']),
('权限管理', '/permission-management', 'Setting', NULL, 6, true, ARRAY['system:admin']);

-- 创建更新updated_at的函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为菜单项表添加触发器
CREATE TRIGGER update_menu_items_updated_at 
    BEFORE UPDATE ON menu_items 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- 为角色表添加触发器
CREATE TRIGGER update_roles_updated_at 
    BEFORE UPDATE ON roles 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();
