package p;
  parameter int I = 3;
  parameter logic [11:0] M = { 4'hA, {(3-I){4'hF, 4'h1}}, 8'h5C };
endpackage
module t;
  localparam logic [11:0] L = { 4'hB, {(3-p::I){4'h7}}, 8'hC3 };
  initial begin #1 $display("A m=%h l=%h", p::M, L); $finish; end
endmodule
