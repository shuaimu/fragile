// Test: Mutex synchronization with pthreads
// Expected: test_mutex() returns 42 (counter incremented exactly 2000 times by 2 threads, verified to be 2000)
// The test returns 42 on success (counter == 2000), 1 on failure

#include <pthread.h>

static int counter = 0;
static pthread_mutex_t mutex;

void* increment(void* arg) {
    for (int i = 0; i < 1000; i++) {
        pthread_mutex_lock(&mutex);
        counter++;
        pthread_mutex_unlock(&mutex);
    }
    return 0;
}

int test_mutex() {
    pthread_mutex_init(&mutex, 0);

    pthread_t t1, t2;
    pthread_create(&t1, 0, increment, 0);
    pthread_create(&t2, 0, increment, 0);

    pthread_join(t1, 0);
    pthread_join(t2, 0);

    pthread_mutex_destroy(&mutex);

    // With proper locking, counter should be exactly 2000
    return (counter == 2000) ? 42 : 1;
}
