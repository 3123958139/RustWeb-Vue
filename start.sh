#!/bin/bash

# 启动后端
echo "启动 Rust 后端..."
cd backend
cargo run &
BACKEND_PID=$!

# 等待后端启动
sleep 5

# 启动前端
echo "启动 Vue 前端..."
cd ../frontend
yarn dev &
FRONTEND_PID=$!

echo "后端 PID: $BACKEND_PID"
echo "前端 PID: $FRONTEND_PID"

# 等待用户中断
echo "按 Ctrl+C 停止服务"
trap "echo '正在停止服务...'; kill $BACKEND_PID $FRONTEND_PID; exit" INT

wait
