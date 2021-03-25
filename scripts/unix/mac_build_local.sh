#!/usr/bin/env sh

set -e

if [ "$#" -ne 3 ]; then
  echo "Expected args: $0 <release-name> <version> <build-dir>"
  exit 1
fi

#if [ "$AWS_ROLE_NAME" != "prod-admin" ]; then
#  echo "prod admin access required; run assume-role prod admin prior to running!"
#  exit 1
#fi

# Verify that fpm exists:
command -v fpm

BASE=$(dirname "$0")
CI_TOOLS=$(realpath "$BASE/../../ci/unix")
BUILD_SCRIPT="$CI_TOOLS/mac_build.sh"

RELEASE_NAME="$1"
VERSION="$2"
BUILD_DIR="$3"
APPLE_IDENTITY=""Developer ID Installer: Joost Wagenaar (5GS9BDM7WS)""
APPLE_CERTIFICATE="Wagenaar_apple_developer_certificate.p12"
S3_CERTIFICATE_LOCATION="pennsieve-cc-operations-use1/$APPLE_CERTIFICATE"
APPLE_CERTIFICATE_PASSWORD=$(aws ssm get-parameters --name ops-apple-developer-id-certificate-password --with-decryption | jq .Parameters[0].Value -r)

# Download the cert to a temporary location:
TEMP_DIR=$(mktemp -d)
aws s3 cp "s3://$S3_CERTIFICATE_LOCATION" "$TEMP_DIR"
LOCAL_CERT="$TEMP_DIR/$APPLE_CERTIFICATE"

# Run the signing package:
$BUILD_SCRIPT \
  "$RELEASE_NAME" \
  "$VERSION" \
  "$BUILD_DIR" \
  "$LOCAL_CERT" \
  "$APPLE_CERTIFICATE_PASSWORD" \
  "$APPLE_IDENTITY"
