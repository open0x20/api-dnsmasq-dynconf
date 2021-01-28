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
                sh 'cat Cargo.toml'
                sh 'cargo test --release'
                sh 'file target/release/dnsmdcd'
                sh 'ldd target/release/dnsmdcd'
                sh 'readelf -e target/release/dnsmdcd'
            }
        }
    }
}