pipeline {
    agent {
        docker {
            image 'rust:latest'
        }
    }
    stages {
        stage('Test') {
            steps {
                echo 'Testing...'
                sh 'cargo test'
            }
        }
    }
    stages {
        stage('Build') {
            steps {
                echo 'Building...'
                sh 'cargo build --release'
            }
        }
    }
}