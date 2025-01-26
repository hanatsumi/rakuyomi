echo "project dir is ${PROJECT_DIR}"

frontendDir="${PROJECT_DIR}/frontend"

koreaderDerivationOutput=$(dirname $(dirname $(readlink -f $(which koreader))))
if [[ $OSTYPE == 'darwin'* ]]; then
  koreaderHome="$koreaderDerivationOutput/Applications/KOReader.app/Contents/koreader"
else
  koreaderHome="$koreaderDerivationOutput/lib/koreader"
fi

cat > "${PROJECT_DIR}/.luarc.json" <<EOF
{
  "\$schema": "https://raw.githubusercontent.com/sumneko/vscode-lua/master/setting/schema.json",
  "workspace.library": [
    "\${3rd}/luassert/library",
    "\${3rd}/busted/library",
    "$koreaderHome/frontend"
  ],
  "runtime.version": "LuaJIT",
  "diagnostics.neededFileStatus": {
    "codestyle-check": "Any"
  }
}
EOF

ln -sf "${PROJECT_DIR}/.luarc.json" "$frontendDir/.luarc.json"
