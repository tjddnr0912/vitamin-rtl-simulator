module top;
  logic [3:0] v; int m; localparam logic [3:0] P = 4'b1?00;
  initial begin
    v = 4'b1000; m = 9;
    case (v) inside
      P: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1100; m = 9;
    case (v) inside
      P: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0100; m = 9;
    case (v) inside
      P: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
