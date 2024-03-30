echo "project dir is ${PROJECT_DIR}"

frontendDir="${PROJECT_DIR}/frontend"

koreaderHome=$(dirname $(dirname $(readlink -f $(which koreader))))
cat > "${PROJECT_DIR}/.luarc.json" <<EOF
{
  "\$schema": "https://raw.githubusercontent.com/sumneko/vscode-lua/master/setting/schema.json",
  "workspace.library": [
    "$koreaderHome/lib/koreader/frontend"
  ],
  "runtime.version": "LuaJIT"
}
EOF

ln -sf "${PROJECT_DIR}/.luarc.json" "$frontendDir/.luarc.json"
