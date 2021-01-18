pipeline {
    agent {
        docker {
            image 'rust:latest'
        }
    }
    stages {
        stage('Build') {
            steps {
                echo 'Building...'
                sh 'cargo build --release'
            }
        }
        stage('Test') {
            steps {
                echo 'Testing...'
                sh 'cargo test'
            }
        }
    }
}