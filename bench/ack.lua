function Ack(M, N)
    if (M == 0) then
        return N + 1
    end
    if (N == 0) then
        return Ack(M - 1, 1)
    end
    return Ack(M - 1, Ack(M, (N - 1)))
end

local time = os.clock()
print(Ack(3,8))
io.write("\n",(os.clock()-time)*1000)
