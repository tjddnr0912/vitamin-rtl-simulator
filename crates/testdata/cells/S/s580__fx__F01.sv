`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b1100; $display("a %b", v inside {4'b1100});
    v = 4'b1100; $display("b %b", v inside {4'b1?00});
    v = 4'b1000; $display("c %b", v inside {4'b1?00});
    v = 4'b0100; $display("d %b", v inside {4'b1?00});
    v = 4'b1100; $display("e %b", v inside {4'b0000, 4'b1?00});
    v = 4'b0101; $display("f %b", v inside {[4'd1:4'd7]});
    v = 4'b1100; if (v inside {4'b1?00}) $display("g then"); else $display("g else");
    v = 4'b1100; $display("h %b", v ==? 4'b1?00);
    v = 4'bx100; $display("i %b", v inside {4'b1?00});
    $finish;
  end
endmodule
