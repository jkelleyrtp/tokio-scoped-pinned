#[tokio::main]
async fn main() {
    // Create a new future that allows internally spawned tasks
    // Requires to be spawned on a !Send runtime
    tokio_local_rt::scope(|c| async move {
        let mut petting = c
            .spawn(async {
                //
                10
            })
            .await;

        c.spawn(async {
            //
        });

        c.spawn(async {
            //
        });
    })
    .await;
}
