package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  if ($bits(q::h(0)) == 4) begin : g initial #1 $display("b=4"); end
  else begin : e initial #1 $display("b!=4"); end
  initial #2 $finish;
  initial #50 $finish;
endmodule
