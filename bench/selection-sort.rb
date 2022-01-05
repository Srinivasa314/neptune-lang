require 'benchmark'
def sortDescending(arr)
    len,i = arr.length,0
    while i<len do
        j = i+1
        while j<len do
            if (arr[i] < arr[j]) then
                t = arr[i]
                arr[i] = arr[j]
                arr[j] = t
            end
            j += 1
        end
        i += 1
    end
end

arr = Array.new(4000)
i = 0
while i < arr.length do
    arr[i] = i
    i += 1
end

time = Benchmark.measure {
sortDescending arr
}
puts arr[0]
puts time.real*1000