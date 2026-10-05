module top;
  int m; logic [3:0] v; localparam logic [3:0] P = 4'b1000;
  initial begin
    v = 0; m = 9;
    case (P) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("m=%0d", m);
    #10 $finish;
  end
endmodule
