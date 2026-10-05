`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("A01 %b", v inside {4'b1100});
    v=4'b1100; $display("A02 %b", v inside {4'b1?00});
    v=4'b1000; $display("A03 %b", v inside {4'b1?00});
    v=4'b0100; $display("A04 %b", v inside {4'b1?00});
    v=4'b1100; $display("A05 %b", v inside {4'b0000, 4'b1?00});
    v=4'b0101; $display("A06 %b", v inside {[4'd1:4'd7]});
    v=4'b1100; $display("A07 %b", v inside {4'b1x00});
    v=4'b1100; $display("A08 %b", v inside {4'b1z00});
    v=4'b0100; $display("A09 %b", v inside {4'b?100});
    v=4'b1100; $display("A10 %b", v inside {4'bx100});
    v=4'b0100; $display("A11 %b", v inside {4'bz100});
    v=4'b1101; $display("A12 %b", v inside {4'b110?});
    v=4'b1010; $display("A13 %b", v inside {4'b????});
    v=4'b1010; $display("A14 %b", v inside {4'bxxxx});
    v=4'b0101; $display("A15 %b", v inside {4'b1?00, [4'd1:4'd7]});
    v=4'b1000; $display("A16 %b", v inside {[4'd1:4'd7], 4'b1?00});
    v=4'b0011; $display("A17 %b", v inside {[4'd1:4'd2], 4'b1?00});
    v=4'b1100; $display("A18 %b", !(v inside {4'b1?00}));
    v=4'b0100; $display("A19 %b", !(v inside {4'b1?00}));
    v=4'b1100; $display("A20 %b", v inside {3'b1?0});
    v=4'b0110; $display("A21 %b", v inside {3'b1?0});
    v=4'b1100; $display("A22 %b", v inside {6'b001?00});
    v=4'b1100; $display("A23 %b", v inside {6'b101?00});
    v=4'b1100; $display("A24 %b", v inside {6'b??1?00});
    v=4'b0110; $display("A25 %b", v inside {4'b0000, 4'b0001, 4'b01?0});
    v=4'b0110; $display("A26 %b", v inside {4'b01?0, 4'b0000});
    v=4'b0111; $display("A27 %b", v inside {4'b01?0, 4'b0000});
    v=4'b1100; $display("A28 %b", v inside {4'hC});
    v=4'b1100; $display("A29 %b", v inside {4'h?});
    v=4'b1100; $display("A30 %b", v inside {4'hx});
    v=4'b1100; $display("A31 %b", v inside {4'dx});
    v=4'b1100; $display("A32 %b", v inside {4'o1?});
    v=4'b0100; $display("A33 %b", v inside {4'b1?00} || v inside {4'b0?00});
    v=4'b0100; $display("A34 %b", (v inside {4'b1?00}) + 2'd1);
    v=4'b1100; $display("A35 %b", {v inside {4'b1?00}, v inside {4'b0?00}});
    v=4'b1100; $display("A36 %b", 4'b1100 inside {4'b1?00});
    v=4'b1100; $display("A37 %b", 4'b0100 inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
