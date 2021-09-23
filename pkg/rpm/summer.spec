Name: $PACKAGE_NAME
Release: 1%{?dist}
Version: $PACKAGE_VERSION
Summary: $PACKAGE_DESCRIPTION
License: $PACKAGE_LICENSE

%description
TODO -- description

%build
cargo build --release

%install
install -t %{buildroot}/usr/bin -s -D -o root -g root target/release/summer

%files
/usr/bin/summer
