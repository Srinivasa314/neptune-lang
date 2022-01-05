require 'benchmark'

def fibonacci( n )
    return  n  if n < 2 
    fibonacci( n - 1 ) + fibonacci( n - 2 )
end
time = Benchmark.measure {
puts fibonacci(33)
}
puts time.real*1000