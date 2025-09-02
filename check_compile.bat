@echo off
echo 正在检查WoPay项目编译状态...
echo.

echo 1. 检查Cargo.toml依赖...
cargo tree --depth=1 2>nul
if %errorlevel% neq 0 (
    echo ❌ Cargo.toml依赖配置有问题
    goto :error
)
echo ✅ 依赖配置正常

echo.
echo 2. 检查语法和类型...
cargo check --quiet
if %errorlevel% neq 0 (
    echo ❌ 编译检查失败
    goto :error
)
echo ✅ 语法和类型检查通过

echo.
echo 3. 尝试编译...
cargo build --quiet
if %errorlevel% neq 0 (
    echo ❌ 编译失败
    goto :error
)
echo ✅ 编译成功

echo.
echo 🎉 所有检查通过！项目可以正常编译。
goto :end

:error
echo.
echo ❌ 检查失败，请查看上面的错误信息。
exit /b 1

:end
echo.
echo 💡 可以运行以下命令启动服务：
echo    cargo run
