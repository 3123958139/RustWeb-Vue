@echo off
echo 启动Rust后端服务器...
start "Backend" cmd /k "cargo run"

echo 等待后端启动...
timeout /t 5 /nobreak > nul

echo 启动Vue前端服务器...
start "Frontend" cmd /k "cd frontend && npm run dev"

echo 服务器启动完成！
echo 后端: http://localhost:3000
echo 前端: http://localhost:5173
pause
