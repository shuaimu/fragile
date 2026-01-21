// Test: Arrays
// Expected: test_arrays() returns 42

int sum_array(int arr[], int size) {
    int sum = 0;
    for (int i = 0; i < size; i = i + 1) {
        sum = sum + arr[i];
    }
    return sum;
}

int test_arrays() {
    int arr[5];
    arr[0] = 5;
    arr[1] = 10;
    arr[2] = 15;
    arr[3] = 7;
    arr[4] = 5;

    return sum_array(arr, 5);  // 5+10+15+7+5 = 42
}
