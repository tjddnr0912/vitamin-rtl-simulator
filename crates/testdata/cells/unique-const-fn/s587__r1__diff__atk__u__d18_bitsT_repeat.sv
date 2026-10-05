module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  typedef logic [f(2):0] T;
  localparam int B = $bits(T);
  int n = 0;
  initial begin repeat ($bits(T)) n++; #1 $display("B=%0d n=%0d", B, n); $finish; end
endmodule
