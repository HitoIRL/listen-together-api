# Listen Together API

Listen Together API is a backend for <a href="https://github.com/HitoIRL/listen-together">Listen Together</a> project. It's written in Rust using Poem framework and Redis as a database.

## Table of Contents
- [ Installation ](#installation)
- [ Usage ](#usage)
    - [ Session Endpoints ](#session-endpoints)
- [ License ](#license)

<a name="installation"></a>
## Installation
1. Clone repository:
```bash
git clone https://github.com/HitoIRL/listen-together-api
```
2. Make sure you have Rust and Cargo installed. If not, you can <a href="https://rustup.rs/">install it from official website</a>
3. Install dependencies and build project:
```bash
cd listen-together-api
cargo build
```
4. Setup your database. Project uses Redis for in-memory session data storing. You can <a href="https://redis.io/download">download it from official website</a> or use Docker:
```bash
docker run --rm -p 6379:6379 redis
```
5. Create YouTube API key. You can <a href="https://developers.google.com/youtube/v3/getting-started">get it from Google Developers Console</a>.
6. Configure environment variables. Rename `.env.example` to `.env` and fill it with your data.
7. Start the server:
```bash
cargo run
```
The API should now be accessible at `http://127.0.0.1:3000`.

<a name="usage"></a>
## Usage
### Session Endpoints
`POST /session`: Creates a new session and returns it's id.<br>
`WS /session/{id}`: Connects with session websocket.

<a name="license"></a>
## License
Listen Together API is licensed under the GNU AGPLv3 license. See the [LICENSE](LICENSE) file for more information.
