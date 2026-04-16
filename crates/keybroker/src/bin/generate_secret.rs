use keybroker::DeploymentSecret;

fn main() {
    println!("{}", DeploymentSecret::random());
}
