package p;
  localparam int K = 4;
  int sink;
  function automatic int g(int a); return a*3; endfunction
  task automatic ta(input logic [31:0] a); $display("ta=%0d", a); endtask
  task ts(input logic [31:0] a); sink = a; $display("ts=%0d", a); endtask
  task automatic to(output logic [31:0] o); o = 32'hFFFF_FFFF; endtask
endpackage
module top;
  import p::ta; import p::ts; import p::to;
  localparam int K = 2;
  function automatic int g(int a); return a; endfunction
  logic [31:0] v;
  initial begin #1 v = 0; ta({g(2){1'b1}}); ts(K'(5'd31)); to(v[g(2):0]); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
