`timescale 1ns/1ns
// R2: the inside operator (IEEE 1800 §11.4.13) must compare integral members with ==?
module repro_inside_op;
    logic [3:0] v;
    initial begin
        v = 4'b1100; #1 $display("a %b inside {4'b1100}         = %b (expect 1)", v, v inside {4'b1100});
        v = 4'b1100; #1 $display("b %b inside {4'b1?00}         = %b (expect 1)", v, v inside {4'b1?00});
        v = 4'b1000; #1 $display("c %b inside {4'b1?00}         = %b (expect 1)", v, v inside {4'b1?00});
        v = 4'b0100; #1 $display("d %b inside {4'b1?00}         = %b (expect 0)", v, v inside {4'b1?00});
        v = 4'b1100; #1 $display("e %b inside {4'b0000,4'b1?00} = %b (expect 1)", v, v inside {4'b0000, 4'b1?00});
        v = 4'b0101; #1 $display("f %b inside {[4'd1:4'd7]}     = %b (expect 1)", v, v inside {[4'd1:4'd7]});
        v = 4'b1100; #1 if (v inside {4'b1?00}) $display("g if-branch taken      (expect this line)");
                        else                    $display("g else-branch taken    (expect the if-branch)");
        v = 4'b1100; #1 $display("h %b ==? 4'b1?00              = %b (expect 1)", v, v ==? 4'b1?00);
`ifndef TWO_STATE
        v = 4'bx100; #1 $display("i %b inside {4'b1?00}         = %b (expect x)", v, v inside {4'b1?00});
`endif
        $finish;
    end
endmodule
