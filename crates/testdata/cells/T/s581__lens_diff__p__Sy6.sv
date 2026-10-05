module top; logic [7:0] arr [((4'bx100 ==? 4'b1?00) & 1'b0) : 0]; initial $display("@udim %0d", $size(arr)); endmodule
