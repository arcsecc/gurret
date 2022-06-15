Master Thesis Work by Sivert Johansen
Thesis link: TODO


# Requirements:
* fuse: `sudo apt-get install fuse`
* rust: See [rustup](https://rustup.rs/)



# filesystem container
Create and mount filesystem container
```bash
ddif=/dev/zero of={your-file} bs=1G count=1$ 
sudo mkfs.ext4 {your-file}
sudo mount -o loop {your-file} {destination-path}
```
Feel free to use different filesystem (E.q btrfs) or different filesystem size

To mount the gurret, set the TARGET in config to {destination-path} and run
```bash
gurret mount
```
