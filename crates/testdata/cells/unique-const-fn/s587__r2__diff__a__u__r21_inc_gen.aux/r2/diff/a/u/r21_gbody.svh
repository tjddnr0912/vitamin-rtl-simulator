    function automatic int f(input int a); return 3; endfunction
    localparam int P = f(2);
    logic [f(2):0] w;
    initial #1 $display("P=%0d bw=%0d", P, $bits(w));
