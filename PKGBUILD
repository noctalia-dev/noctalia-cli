# Maintainer: Your Name <your.email@example.com>
pkgname=noctalia-cli
pkgver=0.1.0
pkgrel=1
pkgdesc="A simple CLI for installing and updating Noctalia components"
arch=('x86_64' 'aarch64')
url="https://github.com/noctalia-dev/noctalia-cli"
license=('MIT' 'Apache')
depends=('glibc' 'gcc-libs')
makedepends=('cargo' 'git')
source=("$pkgname-$pkgver.tar.gz::https://github.com/noctalia-dev/noctalia-cli/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
  cd "$srcdir/$pkgname-v$pkgver"
  cargo fetch --locked
}

build() {
  cd "$srcdir/$pkgname-v$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release
}

check() {
  cd "$srcdir/$pkgname-v$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  cargo test --frozen
}

package() {
  cd "$srcdir/$pkgname-v$pkgver"
  install -Dm755 "target/release/noctalia" "$pkgdir/usr/bin/noctalia"
  
  # Install license if it exists
  if [ -f LICENSE ]; then
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  elif [ -f LICENSE-MIT ]; then
    install -Dm644 LICENSE-MIT "$pkgdir/usr/share/licenses/$pkgname/LICENSE-MIT"
  elif [ -f LICENSE-APACHE ]; then
    install -Dm644 LICENSE-APACHE "$pkgdir/usr/share/licenses/$pkgname/LICENSE-APACHE"
  fi
}

