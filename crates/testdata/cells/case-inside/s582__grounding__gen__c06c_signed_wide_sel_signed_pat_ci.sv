module top;
  logic signed [7:0] v; int m;
  initial begin
    v = 8'shFC; m = 9;
    case (v) inside
      4'sb1?00: m = 1;
      4'b1?11: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'shF8; m = 9;
    case (v) inside
      4'sb1?00: m = 1;
      4'b1?11: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'sh0C; m = 9;
    case (v) inside
      4'sb1?00: m = 1;
      4'b1?11: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'sh08; m = 9;
    case (v) inside
      4'sb1?00: m = 1;
      4'b1?11: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'shFB; m = 9;
    case (v) inside
      4'sb1?00: m = 1;
      4'b1?11: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'sh0B; m = 9;
    case (v) inside
      4'sb1?00: m = 1;
      4'b1?11: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    #10 $finish;
  end
endmodule
