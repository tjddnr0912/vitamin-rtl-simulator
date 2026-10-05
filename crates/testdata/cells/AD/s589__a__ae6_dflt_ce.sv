package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [3:0] h(input int x, input int k = f(2));
    logic [3:0] t;
    h = t ^ k[3:0];
  endfunction
endpackage
module top;
  localparam logic [3:0] P = q::h(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #50 $finish;
endmodule
