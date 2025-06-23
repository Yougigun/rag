import http from 'k6/http';
import { check } from 'k6';

export const options = {
    vus: 1,
    iterations: 1,
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    console.log('🔥 Running smoke test to verify all services are working...\n');

    // Test 1: Health check
    console.log('1️⃣ Testing RAG API health...');
    const healthResponse = http.get(`${BASE_URL}/api/v1/health`);
    
    const healthPassed = check(healthResponse, {
        'API health check passes': (r) => r.status === 200,
        'API returns correct service name': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.service === 'rag-api' && body.status === 'ok';
            } catch (e) {
                return false;
            }
        },
    });

    if (healthPassed) {
        console.log('✓ RAG API is healthy\n');
    } else {
        console.log('✗ RAG API health check failed');
        console.log('Response:', healthResponse.body);
        return;
    }

    // Test 2: Database connectivity (via API)
    console.log('2️⃣ Testing database connectivity...');
    const listResponse = http.get(`${BASE_URL}/api/v1/embedding-tasks`);
    
    const dbPassed = check(listResponse, {
        'Database connection works': (r) => r.status === 200,
        'Database returns array': (r) => {
            try {
                const body = JSON.parse(r.body);
                return Array.isArray(body);
            } catch (e) {
                return false;
            }
        },
    });

    if (dbPassed) {
        console.log('✓ Database connection is working\n');
    } else {
        console.log('✗ Database connection failed');
        console.log('Response:', listResponse.body);
        return;
    }

    // Test 3: External services check
    console.log('3️⃣ Testing external service connectivity...');
    
    // Check Qdrant
    const qdrantResponse = http.get('http://localhost:6333/collections');
    const qdrantPassed = check(qdrantResponse, {
        'Qdrant is accessible': (r) => r.status === 200,
    });

    if (qdrantPassed) {
        console.log('✓ Qdrant vector database is accessible');
    } else {
        console.log('✗ Qdrant vector database is not accessible');
    }

    // Simple CRUD test
    console.log('\n4️⃣ Testing basic CRUD operations...');
    const createPayload = {
        file_name: 'smoke-test.txt'
    };

    const createResponse = http.post(
        `${BASE_URL}/api/v1/embedding-tasks`,
        JSON.stringify(createPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    let taskId;
    const crudPassed = check(createResponse, {
        'Can create embedding task': (r) => r.status === 201,
        'Create returns valid task': (r) => {
            try {
                const body = JSON.parse(r.body);
                taskId = body.id;
                return body.file_name === 'smoke-test.txt' && body.id > 0;
            } catch (e) {
                return false;
            }
        },
    });

    if (crudPassed && taskId) {
        console.log(`✓ CRUD operations working (created task ${taskId})`);
        
        // Clean up
        const deleteResponse = http.del(`${BASE_URL}/api/v1/embedding-tasks/${taskId}`);
        if (deleteResponse.status === 204) {
            console.log('✓ Task cleanup successful');
        }
    } else {
        console.log('✗ CRUD operations failed');
    }

    console.log('\n🎉 Smoke test completed! All core services are operational.');
}