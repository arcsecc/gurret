#[allow(dead_code)]
pub fn mount(target: &str, mountpoint: &str)
{
    // mount -o loop {your-file} {destination-path}
    println!(
        "{}",
        cmd!(&format!("mount -o loop {} {}", target, mountpoint)).stdout_utf8().unwrap()
    );

    /*println!("mount -o loop {} {}", target, mountpoint);
    let s = Command::new("mount")
        .args(["-o loop", target, mountpoint])
        .output()
        .expect("failed to execute process mount");
    println!("{:?}", s);*/
}

#[allow(dead_code)]
pub fn umount(mountpoint: &str)
{
    // sudo umount -l ~/dropbox_folder/
    /*command::new("umount")
    .args(["-l", mountpoint])
    .output()
    .expect("failed to execute process umount");*/
    println!(
        "aha??? {}: {}",
        mountpoint,
        cmd!(&format!("umount -l {}", mountpoint))
            .stdout_utf8()
            .expect("running umount")
    );
}
