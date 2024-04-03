#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import sys
import os
import json
import shutil


def unpack(apktool_path, src_name):

    des = os.path.splitext(src_name)[0]
    if os.path.isfile(des):
        os.remove(des)
    if os.path.isdir(des):
        shutil.rmtree(des)

    command = (
        'java -jar -Xms512m -Xmx1024m "'
        + apktool_path
        + '" --only-main-classes '
        + ' d -f "'
        + src_name
        + '" -o "'
        + des
        + '"'
    )
    print("exec: " + command)
    success = os.system(command)
    if success != 0:
        raise Exception("Failed to unpack apk")

    return des


def pack(apktool_path, src_name):
    if src_name.endswith("/"):
        src_name = src_name[: len(src_name) - 1]

    des = os.path.splitext(src_name)[0] + ".repacked.apk"
    if os.path.isfile(des):
        os.remove(des)

    command = (
        'java -jar -Xms512m -Xmx1024m "'
        + apktool_path
        + '" --only-main-classes '
        + ' b -f "'
        + src_name
        + '" -o "'
        + des
        + '"'
    )
    print("exec: " + command)
    success = os.system(command)
    if success != 0:
        raise Exception("Failed to pack apk")
    return des


def read_config(conf_file_path):
    if os.path.isfile(conf_file_path):
        with open(conf_file_path, mode="rb") as f:
            content = f.read()
        jo = json.loads(content)

        return jo
    else:
        return {"key_path": "", "alias_name": "", "store_pwd": "", "key_pwd": ""}


def sign(apksigner_path, apk_path, conf):

    key_path = conf["key_path"]
    alias_name = conf["alias_name"]
    store_pwd = conf["store_pwd"]
    key_pwd = conf["key_pwd"]

    command = (
        'java -jar "'
        + apksigner_path
        + '" --allowResign --overwrite -ks "'
        + key_path
        + '" --ksPass "'
        + store_pwd
        + '" --ksAlias "'
        + alias_name
        + '" --ksKeyPass "'
        + key_pwd
        + '" -a "'
        + apk_path
        + '"'
    )
    print("exec: " + command)
    success = os.system(command)
    if success != 0:
        raise Exception("Failed to sign apk")


def main():

    argLen = len(sys.argv)

    cmd = ""
    cfg = ""
    path = ""

    idx = 1
    while idx < argLen:
        cmd_s = sys.argv[idx]
        if cmd_s[0] == "-":
            c = cmd_s[1:]
            v = sys.argv[idx + 1]
            if c == "c":
                cmd = v
            elif c == "g":
                cfg = v
            elif c == "f":
                path = v
            idx += 2
        else:
            idx += 1

    if path == "":
        print(
            "using apkex\n\t-c [u: unpack; p: pack;]\n\t-f [file path]\n\t-g [config file path]\n\tto run"
        )
        return

    if not os.path.isabs(path):
        path = os.path.join(os.getcwd(), path)

    exe_folder = os.path.dirname(os.path.realpath(__file__))
    apktool_path = os.path.join(exe_folder, "apktool_2_8_1.jar")
    apksigner_path = os.path.join(exe_folder, "uber-apk-signer-1.3.0.jar")

    if cmd == "u":

        tmp = unpack(apktool_path, path)
        print("\n\nunpack to: " + tmp)

    elif cmd == "p":

        if cfg == "":
            print("using apkex -c p -f [file path] -g [config file path] to pack")
            return

        if not os.path.isabs(cfg):
            cfg = os.path.join(os.getcwd(), cfg)

        cfg_folder = os.path.dirname(cfg)

        conf = read_config(cfg)

        if not os.path.isabs(conf["key_path"]):
            conf["key_path"] = os.path.join(cfg_folder, conf["key_path"])

        if not os.path.isfile(conf["key_path"]):
            raise Exception("key_path not exists")

        packed = pack(apktool_path, path)
        print("\n\npack to: " + packed)
        sign(apksigner_path, packed, conf)
        print("\n\nsign to: " + packed)

    else:
        print(
            "cmd not supported "
            + cmd
            + "\nusing apkex\n\t-c [u: unpack; p: pack;]\n\t-f [file path]\n\t-g [config file path]\n\tto run"
        )


if __name__ == "__main__":
    main()
