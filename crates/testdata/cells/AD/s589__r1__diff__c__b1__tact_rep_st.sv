package p;
  int sink;
  function automatic int g(int a); return a*3; endfunction
  task t(input logic [31:0] a); sink = a; $display("a=%0d", a); endtask
endpackage
module top;
  function automatic int g(int a); return a; endfunction
  initial begin #1 p::t({g(2){1'b1}}); $finish; end
  initial #100 $finish;
endmodule
