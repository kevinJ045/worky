use crate::WorkyRuntime;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

pub struct IsolatePool {
  sender: mpsc::Sender<WorkItem>,
}

struct WorkItem {
  code: String,
  responder: oneshot::Sender<Result<()>>,
}

impl IsolatePool {
  pub fn new(size: usize) -> Self {
    let (sender, receiver) = mpsc::channel::<WorkItem>(32);
    let receiver = Arc::new(Mutex::new(receiver));

    for _ in 0..size {
      let receiver = receiver.clone();
      std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap();

        rt.block_on(async move {
          let mut runtime = WorkyRuntime::new();
          loop {
            let job = {
              let mut rx = receiver.lock().await;
              rx.recv().await
            };

            match job {
              Some(item) => {
                let result = runtime.run(&item.code).await;
                let _ = item.responder.send(result);
              }
              None => break, // Channel closed
            }
          }
        });
      });
    }

    Self { sender }
  }

  pub async fn run(&self, code: String) -> Result<()> {
    let (tx, rx) = oneshot::channel();
    let item = WorkItem {
      code,
      responder: tx,
    };
    self
      .sender
      .send(item)
      .await
      .map_err(|_| anyhow::anyhow!("Pool closed"))?;
    rx.await?
  }
}
