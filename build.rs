fn main() -> shadow_rs::SdResult<()> {
    shadow_rs::ShadowBuilder::builder().build()?;
    embed_resource::compile_for_everything("s7cmd.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
    Ok(())
}
