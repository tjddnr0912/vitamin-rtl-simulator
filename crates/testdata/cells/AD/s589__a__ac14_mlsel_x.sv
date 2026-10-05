package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  logic [31:0] v = 32'h89abcdef; logic [31:0] o;
  initial begin o = v[q::h(2):0]; $display("o=%h", o); #1 $finish; end
  initial #50 $finish;
endmodule
