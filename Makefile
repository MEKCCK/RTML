# RustedTuiMcLauncher (RTML) 构建脚本
# 支持 Linux 和 Windows 交叉编译

# 默认目标
.PHONY: all clean build-linux build-windows

# 构建所有平台
all: build-linux build-windows

# 构建 Linux 版本
build-linux:
	cargo build --release
	@echo "✓ Linux 构建完成: target/release/rtml"

# 构建 Windows 版本（交叉编译）
build-windows:
	cargo build --release --target x86_64-pc-windows-gnu
	@echo "✓ Windows 构建完成: target/x86_64-pc-windows-gnu/release/rtml.exe"

# 安装 Windows 交叉编译工具链
install-cross:
	sudo apt update
	sudo apt install -y gcc-mingw-w64-x86-64
	rustup target add x86_64-pc-windows-gnu
	@echo "✓ Windows 交叉编译工具链已安装"

# 清理构建产物
clean:
	cargo clean
	@echo "✓ 构建产物已清理"

# 打包 Linux 版本
package-linux: build-linux
	mkdir -p dist
	cp target/release/rtml dist/
	cp assets/icon.png dist/
	cp README.md dist/
	cd dist && tar -czf rtml-linux-x86_64.tar.gz rtml icon.png README.md
	@echo "✓ Linux 打包完成: dist/rtml-linux-x86_64.tar.gz"

# 打包 Windows 版本
package-windows: build-windows
	mkdir -p dist
	cp target/x86_64-pc-windows-gnu/release/rtml.exe dist/
	cp assets/icon.png dist/
	cp README.md dist/
	cd dist && zip rtml-windows-x86_64.zip rtml.exe icon.png README.md
	@echo "✓ Windows 打包完成: dist/rtml-windows-x86_64.zip"

# 打包所有平台
package: package-linux package-windows
	@echo "✓ 所有平台打包完成"

# 显示帮助
help:
	@echo "RTML 构建脚本"
	@echo ""
	@echo "可用命令:"
	@echo "  make all              - 构建所有平台"
	@echo "  make build-linux      - 构建 Linux 版本"
	@echo "  make build-windows    - 构建 Windows 版本"
	@echo "  make install-cross    - 安装 Windows 交叉编译工具链"
	@echo "  make clean            - 清理构建产物"
	@echo "  make package-linux    - 打包 Linux 版本"
	@echo "  make package-windows  - 打包 Windows 版本"
	@echo "  make package          - 打包所有平台"
	@echo "  make help             - 显示此帮助"
