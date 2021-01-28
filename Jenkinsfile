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
        stage('Build') {
            steps {
                echo 'Building...'
                sh 'cat Cargo.toml'
                sh 'cargo build --release'
                sh 'file target/release/dnsmdcd'
                sh 'ldd target/release/dnsmdcd'
                sh 'readelf -e target/release/dnsmdcd'
            }
        }
    }
}