do_hash() {
    HASH_NAME=$1
    HASH_CMD=$2
    echo "${HASH_NAME}:"
    for f in $(find -type f); do
        f=$(echo $f | cut -c3-) # remove ./ prefix
        if [ "$f" = "Release" ] || [ "$f" = "Release.gpg" ] || [ "$f" = "InRelease" ] ; then
            continue
        fi
        echo " $(${HASH_CMD} ${f}  | cut -d" " -f1) $(wc -c $f)"
    done
}

cat << EOF
Origin: RLBotGUI
Label: The RLBotGUI Debian package repository
Suite: stable
Codename: stable
Version: 1.0
Architectures: amd64
Components: main
Description: The software repository of the RLBotGUI for Debian-based systems
Date: $(date -Ru)
EOF
do_hash "MD5Sum" "md5sum"
do_hash "SHA1" "sha1sum"
do_hash "SHA256" "sha256sum"