#!/usr/bin/env python3

import subprocess
import os
import sys
import shutil
import time
import errno

G_ADB = ""
G_AAPT = ""
G_DEVICE = ""

exe_folder = os.path.dirname(os.path.realpath(__file__))
bundletool_path = os.path.join(exe_folder, "bundletool-all-1.16.0.jar")


def init_tools():
    global G_ADB
    global G_AAPT
    global G_DEVICE

    android_sdk_path = os.environ.get("ANDROID_HOME") or os.environ.get(
        "ANDROID_SDK_ROOT"
    )
    if android_sdk_path is None or (not os.path.isdir(android_sdk_path)):
        android_sdk_path = "/Users/Shared/Android/sdk"
    if not os.path.isdir(android_sdk_path):
        android_sdk_path = os.path.expanduser("~/Library/Android/sdk")
    if not os.path.isdir(android_sdk_path):
        print("ANDROID_HOME or ANDROID_SDK_ROOT not found")
        exit(1)

    platform_tools_path = os.path.join(android_sdk_path, "platform-tools")

    G_ADB = os.path.join(platform_tools_path, "adb")

    build_tools_path = os.path.join(android_sdk_path, "build-tools")
    build_tools_list = os.listdir(build_tools_path)
    build_tools_list.sort()

    last_build_tool = os.path.join(build_tools_path, build_tools_list[-1])

    if os.path.isdir(last_build_tool):
        G_AAPT = os.path.join(last_build_tool, "aapt")

    os.system(G_ADB + " kill-server")
    os.system(G_ADB + " devices")

    devices_str = run_cmd([G_ADB, "devices"])
    devices = devices_str.split("\n")
    device_list = []
    for d in devices:
        if len(d.strip()) <= 0:
            continue

        dl = d.split("\t")
        if len(dl) != 2:
            continue

        device_list.append(dl[0])

    if len(device_list) == 0:
        print("adblog error: no device found!")
        exit(0)
    elif len(device_list) == 1:
        G_DEVICE = device_list[0]
    elif len(device_list) > 1:
        print("adblog: please select device:")
        idx = 0
        for d in device_list:
            print("\t" + str(idx) + "\t" + d)
            idx = idx + 1
        idx = input("choose:")

        G_DEVICE = device_list[idx]


def run_cmd(cmd):
    # print("run cmd: " + " ".join(cmd))
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out, err = p.communicate()
    if err:
        print(err)
    return out.decode("utf-8")


def self_install(file, des):
    file_path = os.path.realpath(file)

    filename = file_path

    pos = filename.rfind("/")
    if pos:
        filename = filename[pos + 1 :]

    pos = filename.find(".")
    if pos:
        filename = filename[:pos]

    to_path = os.path.join(des, filename)

    print("installing [" + file_path + "] \n\tto [" + to_path + "]")
    if os.path.isfile(to_path):
        os.remove(to_path)

    shutil.copy(file_path, to_path)
    run_cmd(["chmod", "a+x", to_path])


def get_value_by_key(src, prefix, key):
    src = src[len(prefix) + 1 :]
    list = src.split(" ")
    for kvpair in list:
        kvpair = kvpair.strip()
        kvlist = kvpair.split("=")
        if len(kvlist) == 2:
            tmpkey = kvlist[0]
            tmpkey = tmpkey.strip("'")
            tmpkey = tmpkey.strip()
            if tmpkey == key:
                tmpValue = kvlist[1]
                tmpValue = tmpValue.strip("'")
                tmpValue = tmpValue.strip()
                return tmpValue

    return ""


def aab_to_apks(aab_path):
    apks_path = ""
    if aab_path.endswith(".aab"):
        # aab to apks
        apks_path = os.path.splitext(aab_path)[0] + ".apks"
        if os.path.isfile(apks_path):
            os.remove(apks_path)
        print(
            run_cmd(
                [
                    "java",
                    "-jar",
                    bundletool_path,
                    "build-apks",
                    "--bundle=" + aab_path,
                    "--output=" + apks_path,
                ]
            )
        )

    return apks_path


def get_package_and_activity(file_path):
    # aapt dump badging fish-lua-debug.apk
    # package: name='com.by.fishgame' versionCode='2000303' versionName='2.0.3.3' platformBuildVersionName='4.4.2-1456859'
    # launchable-activity: name='org.cocos2dx.lua.AppActivity'  label='' icon=''

    if not os.path.isfile(file_path):
        print("adblog: file not exist!")
        return "", ""

    if file_path.endswith(".apks"):
        apks_unzip = os.path.splitext(file_path)[0] + "_unzip"
        if os.path.isdir(apks_unzip):
            shutil.rmtree(apks_unzip)

        try:
            os.mkdir(apks_unzip)
        except OSError as exc:
            if exc.errno != errno.EEXIST:
                raise

        os.system(
            "unzip -p "
            + file_path
            + " splits/base-master.apk > "
            + os.path.join(apks_unzip, "base-master.apk")
        )

        file_path = os.path.join(apks_unzip, "base-master.apk")

    package_str = ""
    activity_str = ""

    pprefix = "package:"
    aprefix = "launchable-activity:"

    info_str = run_cmd([G_AAPT, "dump", "badging", file_path])
    info_list = info_str.split("\n")
    for pinfo in info_list:

        if pinfo[: len(pprefix)] == pprefix:
            package_str = pinfo

        if pinfo[: len(aprefix)] == aprefix:
            activity_str = pinfo

        if len(package_str) > 0 and len(activity_str) > 0:
            break

    package_name = get_value_by_key(package_str, pprefix, "name")
    activity_name = get_value_by_key(activity_str, aprefix, "name")

    return package_name, activity_name


def adb_get_pid(package_name):
    pid = 0
    limit = 100
    while pid == 0 and limit > 0:
        limit -= 1
        ps_list_str = run_cmd(
            [G_ADB, "-s", G_DEVICE, "shell", "ps", "|", "grep", package_name]
        )
        ps_list_str = ps_list_str.strip()

        if len(ps_list_str):
            ps_list = ps_list_str.split("\n")
            if len(ps_list) > 0:
                for ps_line_str in ps_list:
                    ps_num_list = ps_line_str.split(" ")
                    has_package_name = False
                    for s_str in ps_num_list:
                        s_str = s_str.strip()
                        s_str = s_str.strip("'")
                        s_str = s_str.strip('"')
                        if s_str.lower() == package_name.lower():
                            has_package_name = True
                            break

                    if has_package_name:
                        # print(package_name)
                        # print(ps_num_list)

                        for ps_num_str in ps_num_list:
                            try:
                                pid = int(ps_num_str)
                            except ValueError:
                                pid = 0

                            if pid != 0:
                                break

                    if pid != 0:
                        print(ps_line_str)
                        break

    return pid


def print_help():
    print(
        "adblog "
        "\n\t-c cmd "
        "\n\t\t i install apk, run and log"
        "\n\t\t l read apk package name, run and log"
        "\n\t\t mem install apk if not exist, run and log memory"
        "\n\t\t cpu install apk if not exist, run and log cpu"
        "\n\t-f [file path] "
    )


def __main__():
    # self_install
    if len(sys.argv) > 1 and sys.argv[1] == "install":
        self_install("adblog.py", "/usr/local/bin")
        return

    init_tools()

    argLen = len(sys.argv)

    cmd = ""
    file_path = ""

    idx = 1
    while idx < argLen:
        cmd_s = sys.argv[idx]
        if cmd_s[0] == "-":
            c = cmd_s[1:]
            v = sys.argv[idx + 1]
            if c == "c":
                cmd = v
            elif c == "f":
                file_path = v
            idx += 2
        else:
            idx += 1

    if file_path == "" and cmd == "":
        print_help()
        return

    package_name = ""
    activity_name = ""
    if file_path.endswith(".aab"):
        file_path = aab_to_apks(file_path)
        package_name, activity_name = get_package_and_activity(file_path)
    elif file_path.endswith(".apks"):
        package_name, activity_name = get_package_and_activity(file_path)
    elif file_path.endswith(".apk"):
        package_name, activity_name = get_package_and_activity(file_path)

    if cmd == "i":
        print("uninstalling old apk ...")
        print(run_cmd([G_ADB, "-s", G_DEVICE, "uninstall", package_name]))

        if file_path.endswith(".apks"):
            print("installing new apks ...")
            # bundletool install-apks --apks=/MyApp/my_app.apks
            print(
                run_cmd(
                    [
                        "java",
                        "-jar",
                        bundletool_path,
                        "install-apks",
                        "--adb=" + G_ADB,
                        "--device-id=" + G_DEVICE,
                        "--apks=" + file_path,
                    ]
                )
            )
        else:
            print("installing new apk ...")
            print(run_cmd([G_ADB, "-s", G_DEVICE, "install", file_path]))

        print(
            "adblog: get package name "
            + package_name
            + " activity name "
            + activity_name
        )
        if len(package_name) <= 0 or len(activity_name) <= 0:
            return

        print("adblog: starting process ...")
        activity = package_name + "/" + activity_name
        run_cmd([G_ADB, "-s", G_DEVICE, "shell", "am", "start", "-S", activity])

        pid = adb_get_pid(package_name)

        if pid != 0:
            os.system(G_ADB + " -s " + G_DEVICE + ' logcat -P "" ')
            ps_cmd = (
                G_ADB + " -s " + G_DEVICE + " logcat | grep --color=auto " + str(pid)
            )
            print(ps_cmd)
            os.system(ps_cmd)
        else:
            print("adblog: get pid for " + package_name + " failed!")

    elif cmd == "l":

        print(
            "adblog: get package name "
            + package_name
            + " activity name "
            + activity_name
        )
        if len(package_name) <= 0 or len(activity_name) <= 0:
            return

        print("adblog: starting process ...")
        activity = package_name + "/" + activity_name
        run_cmd([G_ADB, "-s", G_DEVICE, "shell", "am", "start", "-S", activity])

        pid = adb_get_pid(package_name)

        if pid != 0:
            os.system(G_ADB + " -s " + G_DEVICE + ' logcat -P "" ')
            ps_cmd = (
                G_ADB + " -s " + G_DEVICE + " logcat | grep --color=auto " + str(pid)
            )
            print(ps_cmd)
            os.system(ps_cmd)
        else:
            print("adblog: get pid for " + package_name + " failed!")

    elif cmd == "mem":

        print(
            "adblog: get package name "
            + package_name
            + " activity name "
            + activity_name
        )
        if len(package_name) <= 0 or len(activity_name) <= 0:
            return

        pls = run_cmd([G_ADB, "-s", G_DEVICE, "shell", "pm", "list", "packages"])
        pl = pls.split("\n")
        installed = False
        for l in pl:
            if l.strip().endswith(package_name):
                installed = True
                break

        if not installed:
            if file_path.endswith(".apks"):
                print("installing new apks ...")
                # bundletool install-apks --apks=/MyApp/my_app.apks
                print(
                    run_cmd(
                        [
                            "java",
                            "-jar",
                            bundletool_path,
                            "install-apks",
                            "--adb=" + G_ADB,
                            "--device-id=" + G_DEVICE,
                            "--apks=" + file_path,
                        ]
                    )
                )
            else:
                print("installing new apk ...")
                print(run_cmd([G_ADB, "-s", G_DEVICE, "install", file_path]))

        print("adblog: starting process ...")
        activity = package_name + "/" + activity_name
        run_cmd([G_ADB, "-s", G_DEVICE, "shell", "am", "start", "-S", activity])

        pid = adb_get_pid(package_name)

        if pid != 0:
            while True:
                print(
                    run_cmd(
                        [G_ADB, "-s", G_DEVICE, "shell", "dumpsys", "meminfo", str(pid)]
                    )
                )
                time.sleep(2)
        else:
            print("adblog: get pid for " + package_name + " failed!")

    elif cmd == "cpu":

        print(
            "adblog: get package name "
            + package_name
            + " activity name "
            + activity_name
        )
        if len(package_name) <= 0 or len(activity_name) <= 0:
            return

        pls = run_cmd([G_ADB, "-s", G_DEVICE, "shell", "pm", "list", "packages"])
        pl = pls.split("\n")
        installed = False
        for l in pl:
            if l.strip().endswith(package_name):
                installed = True
                break

        if not installed:
            if file_path.endswith(".apks"):
                print("installing new apks ...")
                # bundletool install-apks --apks=/MyApp/my_app.apks
                print(
                    run_cmd(
                        [
                            "java",
                            "-jar",
                            bundletool_path,
                            "install-apks",
                            "--adb=" + G_ADB,
                            "--device-id=" + G_DEVICE,
                            "--apks=" + file_path,
                        ]
                    )
                )
            else:
                print("installing new apk ...")
                print(run_cmd([G_ADB, "-s", G_DEVICE, "install", file_path]))

        print("adblog: starting process ...")
        activity = package_name + "/" + activity_name
        run_cmd([G_ADB, "-s", G_DEVICE, "shell", "am", "start", "-S", activity])

        pid = adb_get_pid(package_name)

        if pid != 0:
            while True:
                print(
                    run_cmd(
                        [
                            G_ADB,
                            "-s",
                            G_DEVICE,
                            "shell",
                            "dumpsys",
                            "cpuinfo",
                            "|",
                            "grep",
                            package_name,
                        ]
                    )
                )
                time.sleep(2)
        else:
            print("adblog: get pid for " + package_name + " failed!")
    else:
        print_help()


__main__()
