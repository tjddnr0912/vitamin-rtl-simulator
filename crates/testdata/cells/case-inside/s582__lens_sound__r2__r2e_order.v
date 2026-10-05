module m_after_var;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd2; case (x) inside[1:0]: m = 1; default: m = 0; endcase $display("m_after_var m=%0d", m); end
  reg [3:0] inside;
  initial inside = 4'b0110;
endmodule
module m_after_gen;
  reg [3:0] x; integer m;
  generate if (1) begin : g
    initial begin #1 x = 4'd2; case (x) inside[1:0]: m = 1; default: m = 0; endcase $display("m_after_gen m=%0d", m); end
  end endgenerate
  reg [3:0] inside;
  initial inside = 4'b0110;
endmodule
module m_task_formal;
  integer m;
  task t; input [3:0] inside; begin #1; case (4'd2) inside[1:0]: m = 1; default: m = 0; endcase end endtask
  initial begin t(4'b0110); $display("m_task_formal m=%0d", m); end
endmodule
module m_auto_formal;
  integer m;
  function automatic [3:0] f; input [3:0] inside; input integer n;
    begin if (n > 0) f = f(inside, n - 1); else begin case (4'd2) inside[1:0]: f = 1; default: f = 0; endcase end end
  endfunction
  initial begin #1 m = f(4'b0110, 2); $display("m_auto_formal m=%0d", m); end
endmodule
module top; m_after_var a(); m_after_gen b(); m_task_formal c(); m_auto_formal d(); initial #10 $finish; endmodule
