# Maintainer: MEKCCK <3504638270@qq.com>
pkgname=rtml
pkgver=0.1.0
pkgrel=2
pkgdesc="RTML - Rust TUI Minecraft Launcher"
arch=('x86_64')
url="https://github.com/MEKCCK/RTML"
license=('GPL3')
depends=('gcc-libs' 'java-runtime>=17')
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/MEKCCK/RTML/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/RTML-$pkgver"
    cargo build --release --frozen
}

package() {
    cd "$srcdir/RTML-$pkgver"
    install -Dm755 target/release/rtml "$pkgdir/usr/bin/rtml"
    install -Dm644 assets/rtml.desktop "$pkgdir/usr/share/applications/rtml.desktop"
    install -Dm644 assets/icon.png \
      "$pkgdir/usr/share/icons/hicolor/256x256/apps/rtml.png"
    install -Dm644 assets/icon.png \
      "$pkgdir/usr/share/icons/hicolor/512x512/apps/rtml.png"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
