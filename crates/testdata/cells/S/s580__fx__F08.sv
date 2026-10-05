`timescale 1ns/1ns
module t;
  logic [3:0] v; integer n;
  initial begin
    v = 4'b1100;
    if (v inside {4'b1?00}) $display("if then"); else $display("if else");
    $display("ternary %h", (v inside {4'b1?00}) ? 8'd1 : 8'd2);
    case (1'b1) (v inside {4'b1?00}): $display("case item"); default: $display("case default"); endcase
    assert (v inside {4'b1?00}) $display("assert pass"); else $display("assert fail");
    v = 4'b1000; while (v inside {4'b1?00}) v = v + 4'd1; $display("while %b", v);
    n = 0; for (v = 4'b1000; v inside {4'b1?0?}; v = v + 4'd1) n = n + 1; $display("for %0d", n);
    $finish;
  end
endmodule
