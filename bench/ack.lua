function Ack(M, N)
    if (M == 0) then
        return N + 1
    end
    if (N == 0) then
        return Ack(M - 1, 1)
    end
    return Ack(M - 1, Ack(M, (N - 1)))
end

print(Ack(3,8))
