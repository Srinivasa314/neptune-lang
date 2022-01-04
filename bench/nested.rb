require 'benchmark'

def runLoop()
    counter = 0
    a = 0
    while a < 75 do
        b = 0
        while b < 75 do
            c = 0
            while c < 75 do
                d = 0
                while d < 75 do
                    counter += 1
                    d += 1
                end
                c += 1
            end
            b += 1
        end
        a += 1
    end
    return counter
end

time = Benchmark.measure {
    puts runLoop
}
puts time.real*1000