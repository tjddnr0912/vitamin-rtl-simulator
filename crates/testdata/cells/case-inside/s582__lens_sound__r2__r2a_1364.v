module s_port(input [3:0] inside);
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd2; case (x) inside[1:0]: m = 1; default: m = 0; endcase $display("s_port m=%0d", m); end
endmodule
module s_genvar;
  reg [3:0] x; integer m;
  genvar inside;
  generate for (inside = 2; inside < 3; inside = inside + 1) begin : g
    initial begin #1 x = 4'd2; case (x) inside + 0: m = 1; default: m = 0; endcase $display("s_genvar m=%0d", m); end
  end endgenerate
endmodule
module s_gen_net;
  reg [3:0] x; integer m;
  generate if (1) begin : g
    reg [3:0] inside;
    initial begin inside = 4'b0110; #1 x = 4'd2; case (x) inside[1:0]: m = 1; default: m = 0; endcase $display("s_gen_net m=%0d", m); end
  end endgenerate
endmodule
module s_param_real;
  parameter real inside = 2.0;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd2; case (x) inside + 0: m = 1; default: m = 0; endcase $display("s_param_real m=%0d", m); end
endmodule
module s_param_str;
  parameter inside = "b";
  reg [7:0] x; integer m;
  initial begin #1 x = 8'h62; case (x) inside + 0: m = 1; default: m = 0; endcase $display("s_param_str m=%0d", m); end
endmodule
module s_param_wide;
  parameter [99:0] inside = 100'h5;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd5; case (x) inside + 0: m = 1; default: m = 0; endcase $display("s_param_wide m=%0d", m); end
endmodule
module s_specparam;
  specparam inside = 5;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd5; case (x) inside + 0: m = 1; default: m = 0; endcase $display("s_specparam m=%0d", m); end
endmodule
module s_func_after;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase $display("s_func_after m=%0d", m); end
  function [3:0] inside; input [3:0] a; inside = a + 1; endfunction
endmodule
module s_func_gen;
  reg [3:0] x; integer m;
  generate if (1) begin : g
    function [3:0] inside; input [3:0] a; inside = a + 1; endfunction
    initial begin #1 x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase $display("s_func_gen m=%0d", m); end
  end endgenerate
endmodule
module s_formal;
  integer m;
  function [3:0] f; input [3:0] inside; begin case (4'd2) inside[1:0]: f = 1; default: f = 0; endcase end endfunction
  initial begin #1 m = f(4'b0110); $display("s_formal m=%0d", m); end
endmodule
module s_task_local;
  integer m;
  task t; reg [3:0] inside; begin inside = 4'b0110; case (4'd2) inside[1:0]: m = 1; default: m = 0; endcase end endtask
  initial begin #1 t; $display("s_task_local m=%0d", m); end
endmodule
module top;
  s_port u0(.inside(4'b0110));
  s_genvar u1(); s_gen_net u2(); s_param_real u3(); s_param_str u4(); s_param_wide u5();
  s_specparam u6(); s_func_after u7(); s_func_gen u8(); s_formal u9(); s_task_local u10();
  initial #10 $finish;
endmodule
