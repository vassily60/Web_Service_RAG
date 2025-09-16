# What is Podman?

An introduction to Podman, a containerization tool that provides a Docker-compatible CLI for managing containers and images:
https://www.redhat.com/en/topics/containers/what-is-podman#:~:text=Podman%20vs.,-Docker&text=Docker%20is%20a%20containerization%20technology,mode%20to%20its%20daemon%20configuration.

# Using in AWS ECR
https://docs.aws.amazon.com/AmazonECR/latest/userguide/Podman.html

# Getting Started with Podman on macOS

Podman does not run natively on macOS since it relies on Linux kernel features such as namespaces and cgroups. However, you can use Podman on macOS by running it inside a virtualized Linux environment via **Podman Machine**.

### Steps to Install and Start Podman on macOS:

1. **Install Podman using Homebrew**:
   ```sh
   brew install podman
   ```

2. **Initialize and Start Podman Machine (VM for Podman on macOS)**:
   ```sh
   podman machine init
   podman machine start
   ```

3. **Verify Installation**:
   ```sh
   podman version
   podman info
   podman ps  # Should show an empty list if no containers are running
   ```

4. **Run a Test Container**:
   ```sh
   podman run --rm -it alpine sh
   ```
   This command will pull the **Alpine Linux** image (if not already present) and run an interactive shell.

5. **Stop Podman Machine (If Needed)**:
   ```sh
   podman machine stop
   ```

6. **Remove Podman Machine (If Needed)**:
   ```sh
   podman machine rm
   ```

### Additional Considerations:
- Since Podman runs inside a Linux virtual machine (`podman machine`), filesystem operations and networking may behave differently from native Linux environments.
- To run rootless containers, you can simply use `podman` instead of `sudo podman`.