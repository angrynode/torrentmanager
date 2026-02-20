use static_serve::embed_assets;

// Embed the assets in the binary, generating the static_router function
embed_assets!("assets", allow_unknown_extensions = true);
