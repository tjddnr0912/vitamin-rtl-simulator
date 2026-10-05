`timescale 1ns/1ns
// R3/R4: unique-case violation reports (IEEE 1800 §12.5.3)
module repro_unique;
    logic [1:0] r;
    logic       y;
    always_comb begin
        y = 1'b0;
        unique casez (r)
            2'b?1: y = 1'b1;
            2'b1?: y = 1'b0;
        endcase
    end
    initial begin
        r = 2'b01; #1;   // t=0: legal value set in the same time step as the first always_comb pass
        r = 2'b11; #1;   // t=1: both items match -> overlap violation expected
        r = 2'b00; #1;   // t=2: no item matches -> no-match violation expected
        $finish;
    end
endmodule
