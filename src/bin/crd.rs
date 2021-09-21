use kube::core::crd::v1::CustomResourceExt;
use ready_image::ReadyImage;

fn main() {
    println!("{}", serde_yaml::to_string(&ReadyImage::crd()).unwrap())
}
