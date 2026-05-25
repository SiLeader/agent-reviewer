// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct ReActAgentNote {
    session_id: String,
    agent_id: String,
}

#[derive(Clone)]
pub struct ReActAgentNoteManager {
    session_id: String,
}

impl ReActAgentNoteManager {
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }

    pub fn create_note(&self, agent_id: String) -> ReActAgentNote {
        ReActAgentNote::new(self.session_id.clone(), agent_id)
    }
}

impl ReActAgentNote {
    pub fn new(session_id: String, agent_id: String) -> Self {
        Self {
            session_id,
            agent_id,
        }
    }

    fn note_file(&self) -> PathBuf {
        PathBuf::from(format!(
            ".agent-reviewer/notes/{}/{}.txt",
            self.session_id, self.agent_id
        ))
    }

    async fn open(&self) -> anyhow::Result<File> {
        let path = self.note_file();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let file = File::create(path).await?;
        Ok(file)
    }

    pub async fn write(&self, new_note: String) -> anyhow::Result<()> {
        let mut file = self.open().await?;
        file.write_all(new_note.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }
}
