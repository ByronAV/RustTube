# RustTube

RustTube is a full-stack video streaming application designed to showcase the capabilities of Rust on the backend and React on the frontend. It offers a modular architecture suitable for learning, experimentation, or as a foundation for more advanced streaming platforms.

---

## Features

- **Backend**: Built with Rust, providing a robust and high-performance server.
- **Frontend**: In development, build in React, user should be able to watch videos, upload videos and delete videos.
- **Database Integration**: Handles video metadata and user information efficiently.
- **Video Storage**: Manages video files and streaming capabilities.
- **Recommendation System**: Suggests videos based on user interactions.
- **Dockerized Setup**: Utilizes Docker Compose for easy deployment and environment management.

---

## Project Structure

The repository is organized into the following directories:

- **`backend/`**: Rust-based server handling API requests, authentication, and business logic.
- **`frontend/`**: React application providing the user interface and client-side logic.
- **`db-fixtures/`**: Contains initial data and scripts for database seeding.
- **`storage/`**: Manages video file storage and retrieval mechanisms.
- **`scripts/`**: Utility scripts for setup, deployment, and maintenance tasks.
- **`docker-compose.yml` & `docker-compose-prod.yml`**: Docker Compose configurations for development and production environments.

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) and [npm](https://www.npmjs.com/)
- [Docker](https://www.docker.com/) and [Docker Compose](https://docs.docker.com/compose/)

### Development Setup

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/ByronAV/RustTube.git
   cd RustTube
   ```

2. **Start Services with Docker Compose**:

    ```bash
    docker-compose up --build -d
    ```

    This command builds and starts all necessary services, including the backend, frontend, and database.

3. **Access the Application**:

    At this time, you can access the application by reaching here:

    ```bash
    localhost:3000/video?<ID_of_video>
    ```

    This will retrieve the video and play it in the browser. We are working towards creating a Frontend UI in React and Typescript to be able to upload videos, watch videos and delete videos.
