package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  int sink;
  function automatic int g(); return 8; endfunction
  task automatic t1(output logic [31:0] o); o = {g(){1'b1}}; endtask
  task automatic t2(output logic [31:0] o); o = {h(){1'b1}}; endtask
  task t3(output logic [31:0] o); sink = 1; o = {g(){1'b1}}; endtask
  task t4(output logic [31:0] o); sink = 2; o = {h(){1'b1}}; endtask
endpackage
module top;
  import r::*;
  import p::t1; import p::t2; import p::t3; import p::t4;
  function automatic int g(); return 5; endfunction
  logic [31:0] a, b, c, d;
  initial begin #1 t1(a); t2(b); t3(c); t4(d); $display("a=%0d b=%0d c=%0d d=%0d", a, b, c, d); $finish; end
  initial #100 $finish;
endmodule
