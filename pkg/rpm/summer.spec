Name: $PACKAGE_NAME
Release: 1%{?dist}
Version: $PACKAGE_VERSION
Summary: $PACKAGE_DESCRIPTION
License: $PACKAGE_LICENSE

%description
Summer is an application that reads the contents of a directory, and
generates a summary based on a custom configuration.

%build
cargo build --release

%install
install -t %{buildroot}/usr/bin -s -D -o root -g root target/release/summer

%files
/usr/bin/summer
