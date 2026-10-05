package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  logic [7:0] v = 8'hff;
  logic [7:0] o;
  initial begin o = 8'(q::h(2)'(v)); $display("o=%h", o); #1 $finish; end
  initial #50 $finish;
endmodule
