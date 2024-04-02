#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import sys
import os
import shutil
import subprocess
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.serialization.pkcs7 import (
    load_der_pkcs7_certificates,
)

ENCODING = "utf-8"


def run_cmd(cmd):
    # print("-------------------")

    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out, err = p.communicate()
    return_code = p.returncode
    # print("return : " + str(return_code))
    if return_code != 0:
        str_cmd = " ".join(cmd)
        print("run cmd error: " + str_cmd)
        if err:
            print(err.decode(ENCODING))
        raise Exception("error: " + str_cmd + " - " + str(return_code))

    # print("-------------------\n")
    return out


def main():

    if len(sys.argv) <= 1:
        print("Usage: apkinfo [apk file path]")
        return

    exe_path = os.path.dirname(os.path.realpath(__file__))
    apksigner_path = os.path.join(exe_path, "apksigner_34_0_0.jar")
    apktool_path = os.path.join(exe_path, "apktool_2_8_1.jar")

    apk_path = sys.argv[1]

    if not os.path.isabs(apk_path):
        apk_path = os.path.join(os.getcwd(), apk_path)

    if not os.path.isfile(apk_path):
        print("file not found")
        return

    des = os.path.splitext(apk_path)[0]
    if os.path.isdir(des):
        shutil.rmtree(des)

    print("extracting apk...\n\n")
    run_cmd(
        ["java", "-jar", apktool_path, "d", apk_path, "--only-main-classes", "-o", des]
    )

    unpacked = des

    manifest_path = os.path.join(unpacked, "AndroidManifest.xml")

    with open(manifest_path, "r") as f:
        manifest = f.read()
        package_start = manifest.index('package="') + len('package="')
        package_end = manifest.index('"', package_start)
        package = manifest[package_start:package_end]
        print("Package:")
        print("=====================")
        print(package)
        print("=====================")

    meta_path = os.path.join(unpacked, "original", "META-INF")

    if not os.path.isdir(meta_path):
        raise Exception("META-INF not found")

    cert_list = os.listdir(meta_path)
    for cert in cert_list:
        if cert.endswith(".RSA"):
            cert_rsa = os.path.join(meta_path, cert)
            break

    apk_results = run_cmd(
        ["java", "-jar", apksigner_path, "verify", "--print-certs", "-v", apk_path]
    )
    print(apk_results.decode(ENCODING))

    with open(cert_rsa, "rb") as f:
        cert_bytes = f.read()

    certs = load_der_pkcs7_certificates(cert_bytes)

    if len(certs) == 0:
        raise Exception("cert not found")

    index = 0
    for cert in certs:
        index += 1
        print("Modulus - " + str(index) + ":")
        print("=====================")
        print(str(cert.public_key().public_numbers().n))
        print("=====================")

        md5 = cert.fingerprint(hashes.MD5()).hex().upper()
        md5 = " ".join(md5[i : i + 2] for i in range(0, len(md5), 2))
        print("MD5 - " + str(index) + ":")
        print("=====================")
        print(md5)
        print("=====================")

        sha1 = cert.fingerprint(hashes.SHA1()).hex().upper()
        sha1 = " ".join(sha1[i : i + 2] for i in range(0, len(sha1), 2))
        print("SHA1 - " + str(index) + ":")
        print("=====================")
        print(sha1)
        print("=====================")

        sha256 = cert.fingerprint(hashes.SHA256()).hex().upper()
        sha256 = " ".join(sha256[i : i + 2] for i in range(0, len(sha256), 2))
        print("SHA256 - " + str(index) + ":")
        print("=====================")
        print(sha256)
        print("=====================")

    if os.path.exists(unpacked):
        shutil.rmtree(unpacked, ignore_errors=True)


if __name__ == "__main__":
    main()
