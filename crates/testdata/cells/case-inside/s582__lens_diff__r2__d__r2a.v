`timescale 1ns/1ns
module f_formal(output reg [3:0] m);
  function [3:0] f1;
    input [3:0] v; input [3:0] inside;
    begin
      case (v) inside[2:1]: f1 = 1; default: f1 = 0; endcase
    end
  endfunction
  initial m = f1(4'd3, 4'b0110);
endmodule
module f_local(output reg [3:0] m);
  function [3:0] f2;
    input [3:0] v;
    reg [3:0] inside;
    begin
      inside = 4'b0110;
      case (v) inside + 1: f2 = 1; default: f2 = 0; endcase
    end
  endfunction
  initial m = f2(4'd7);
endmodule
module f_auto(output reg [3:0] m);
  function automatic [3:0] f3;
    input [3:0] v; input integer d;
    reg [3:0] inside;
    begin
      inside = 4'b0110;
      if (d > 0) f3 = f3(v, d - 1);
      else case (v) inside + 1: f3 = 1; default: f3 = 0; endcase
    end
  endfunction
  initial m = f3(4'd7, 2);
endmodule
module f_nblk(output reg [3:0] m);
  function [3:0] f4;
    input [3:0] v;
    begin : fb
      reg [3:0] inside;
      inside = 4'b0110;
      case (v) inside + 1: f4 = 1; default: f4 = 0; endcase
    end
  endfunction
  initial m = f4(4'd7);
endmodule
module t_formal(output reg [3:0] m);
  task t5;
    input [3:0] v; input [3:0] inside; output [3:0] r;
    begin
      case (v) inside[2:1]: r = 1; default: r = 0; endcase
    end
  endtask
  initial t5(4'd3, 4'b0110, m);
endmodule
module t_local(output reg [3:0] m);
  task t6;
    input [3:0] v; output [3:0] r;
    reg [3:0] inside;
    begin
      inside = 4'b0110;
      case (v) inside + 1: r = 1; default: r = 0; endcase
    end
  endtask
  initial t6(4'd7, m);
endmodule
module p_mod #(parameter inside = 4'd6) (output reg [3:0] m);
  reg [3:0] x;
  initial begin x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
endmodule
module lp_mod(output reg [3:0] m);
  localparam inside = 4'd6;
  reg [3:0] x;
  initial begin x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
endmodule
module sp_mod(output reg [3:0] m);
  specparam inside = 6;
  reg [3:0] x;
  initial begin x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
endmodule
module gv_mod(output reg [3:0] m);
  genvar inside;
  reg [3:0] x;
  generate for (inside = 6; inside < 7; inside = inside + 1) begin : g
    initial begin x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
  end endgenerate
endmodule
module gb_mod(output reg [3:0] m);
  reg [3:0] x;
  generate if (1) begin : g
    reg [3:0] inside;
    initial begin inside = 4'd6; x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
  end endgenerate
endmodule
module nb_mod(output reg [3:0] m);
  reg [3:0] x;
  initial begin : outer
    reg [3:0] inside;
    inside = 4'd6; x = 4'd7;
    begin : inner
      case (x) inside + 1: m = 1; default: m = 0; endcase
    end
  end
endmodule
module im_mod(output reg [3:0] m);
  reg [3:0] x;
  assign inside = 1'b1;
  initial begin #1 x = 4'd2; case (x) inside + 1: m = 1; default: m = 0; endcase end
endmodule
module ubd_mod(output reg [3:0] m);
  reg [3:0] x;
  initial begin #1 x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
  reg [3:0] inside;
  initial inside = 4'd6;
endmodule
module port_mod(input [3:0] inside, output reg [3:0] m);
  reg [3:0] x;
  initial begin #1 x = 4'd7; case (x) inside + 1: m = 1; default: m = 0; endcase end
endmodule
module top;
  wire [3:0] a1, a2, a3, a4, a5, a6, b1, b2, b3, b4, b5, b6, b7, b8, b9;
  f_formal u1(a1); f_local u2(a2); f_auto u3(a3); f_nblk u4(a4); t_formal u5(a5); t_local u6(a6);
  p_mod u7(b1); lp_mod u8(b2); sp_mod u9(b3); gv_mod u10(b4); gb_mod u11(b5); nb_mod u12(b6);
  im_mod u13(b7); ubd_mod u14(b8); port_mod u15(4'd6, b9);
  initial begin #2 $display("ff=%0d fl=%0d fa=%0d fn=%0d tf=%0d tl=%0d p=%0d lp=%0d sp=%0d gv=%0d gb=%0d nb=%0d im=%0d ubd=%0d port=%0d", a1, a2, a3, a4, a5, a6, b1, b2, b3, b4, b5, b6, b7, b8, b9); $finish; end
  initial #1000 $finish;
endmodule
